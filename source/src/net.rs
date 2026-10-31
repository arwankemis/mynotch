//! HTTPS transport with async WinHTTP completion and cancellation. Call from a module-owned worker.
//! WinHTTP validates certificates and handles TLS; requests never follow redirects.
use std::time::{Duration, Instant};
use std::sync::{Arc,Mutex,atomic::{AtomicBool,Ordering},mpsc};
use std::cell::RefCell;
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Clone,Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation{pub fn cancel(&self){self.0.store(true,Ordering::Release);}pub fn is_cancelled(&self)->bool{self.0.load(Ordering::Acquire)}}
thread_local!{static CANCEL:RefCell<Option<Cancellation>>=const{RefCell::new(None)};}
pub fn with_cancel<T>(token:Cancellation,work:impl FnOnce()->T)->T{
    struct Restore(Option<Cancellation>);impl Drop for Restore{fn drop(&mut self){CANCEL.with(|c|*c.borrow_mut()=self.0.take());}}
    let _restore=Restore(CANCEL.with(|c|c.replace(Some(token))));work()
}
pub fn check_cancel()->Result<(),String>{if CANCEL.with(|c|c.borrow().as_ref().is_some_and(Cancellation::is_cancelled)){Err("Pekerjaan dibatalkan.".into())}else{Ok(())}}

pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Hosts explicitly allowed to bypass the IP/private-range block, from
/// `Settings::net_allowed_hosts` (edited via settings.json; no settings UI yet). Set once
/// at startup; empty by default, which blocks every IP literal and private range.
/// LazyLock because HashSet::new is not const, so it cannot initialize a plain static.
static ALLOWED_HOSTS: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// Replaces the host allowlist (entries are lowercased and trimmed on the way in).
pub fn set_allowed_hosts(hosts: &[String]) {
    if let Ok(mut allowed) = ALLOWED_HOSTS.lock() {
        *allowed = hosts.iter().map(|h| h.trim().to_ascii_lowercase()).filter(|h| !h.is_empty()).collect();
    }
}

fn host_allowed(host: &str, allowed: &HashSet<String>) -> bool {
    if allowed.contains(&host.to_ascii_lowercase()) {
        return true;
    }
    // An allowlist entry may spell the same IP differently (notably IPv6); compare
    // parsed addresses too so an allowlisted IP is not defeated by its own spelling.
    if let Ok(ip) = host.parse::<IpAddr>() {
        return allowed.iter().any(|entry| entry.parse::<IpAddr>().map(|e| e == ip).unwrap_or(false));
    }
    false
}

/// True for IP literals in ranges that must never be reached from a mod or a fetched feed
/// unless explicitly allowlisted: loopback, private (RFC 1918), link-local, unspecified,
/// broadcast, multicast, benchmarking, documentation, and reserved space, for both IPv4
/// and IPv6 (including IPv4-mapped IPv6). DNS names are not literals and are not blocked
/// here; a name that resolves to a private address (DNS rebinding) is a known residual.
fn host_is_blocked(host: &str) -> bool {
    let ip: IpAddr = match host.parse() {
        Ok(ip) => ip,
        Err(_) => return false,
    };
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            let is_benchmarking = octets[0] == 198 && (octets[1] & 0xfe) == 18;
            let is_documentation = matches!(octets, [192, 0, 2, _] | [198, 51, 100, _] | [203, 0, 113, _]);
            let is_reserved = octets[0] >= 240;
            v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified()
                || v4.is_broadcast() || v4.is_multicast() || is_reserved
                || is_benchmarking || is_documentation
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return host_is_blocked(&v4.to_string());
            }
            v6.is_loopback() || v6.is_unspecified() || v6.is_multicast()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique-local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpsUrl {
    host: String,
    port: u16,
    resource: String,
}

fn parse_url(url: &str) -> Result<HttpsUrl, String> {
    if url.len() > 16384 || !url.is_ascii() || url.bytes().any(|b| b <= 32 || b == 127) {
        return Err("URL tidak valid atau terlalu panjang.".into());
    }
    let rest = url.strip_prefix("https://").ok_or("URL harus menggunakan HTTPS.")?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') || rest.contains('#') || authority.contains('\\') {
        return Err("URL HTTPS tidak valid; userinfo dan fragment tidak didukung.".into());
    }
    let (host, port) = if authority.starts_with('[') {
        let close = authority.find(']').ok_or("Alamat IPv6 tidak valid.")?;
        let host = &authority[1..close];
        let tail = &authority[close + 1..];
        let port = if tail.is_empty() { 443 } else { tail.strip_prefix(':').ok_or("Port tidak valid.")?.parse::<u16>().map_err(|_| "Port tidak valid.")? };
        if host.is_empty() || !host.bytes().all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.') { return Err("Alamat IPv6 tidak valid.".into()); }
        (host, port)
    } else {
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, value)) => (host, value.parse::<u16>().map_err(|_| "Port tidak valid.")?),
            None => (authority, 443),
        };
        if host.is_empty() || !host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-') { return Err("Nama host tidak valid; gunakan hostname ASCII/punycode.".into()); }
        (host, port)
    };
    if port == 0 { return Err("Port tidak valid.".into()); }
    let tail = &rest[authority_end..];
    let resource = if tail.is_empty() { "/".to_owned() } else if tail.starts_with('?') { format!("/{tail}") } else { tail.to_owned() };
    Ok(HttpsUrl { host: host.into(), port, resource })
}

pub fn get(url: &str) -> Result<Vec<u8>, String> { request("GET", url, &[], &[]) }

pub fn request(method: &str, url: &str, headers: &[(&str, &str)], body: &[u8]) -> Result<Vec<u8>, String> {
    check_cancel()?;
    let parsed = parse_url(url)?;
    // Transport-level guard, so every caller (built-in feeds and mod Net alike) is held
    // to it: IP literals and private/loopback/link-local ranges are refused unless the
    // host is explicitly listed in net_allowed_hosts.
    if host_is_blocked(&parsed.host) {
        let allowed = ALLOWED_HOSTS.lock().map(|set| host_allowed(&parsed.host, &set)).unwrap_or(false);
        if !allowed {
            return Err("Host tujuan diblokir: alamat IP literal atau jaringan privat/loopback hanya diizinkan bila tercantum di net_allowed_hosts.".into());
        }
    }
    if !matches!(method, "GET" | "POST") { return Err("Metode HTTP tidak didukung.".into()); }
    if body.len() > MAX_REQUEST_BYTES { return Err("Request melebihi batas ukuran.".into()); }
    for (name, value) in headers {
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') || value.contains(['\r', '\n', '\0']) || value.len() > 16384 {
            return Err("Header HTTP tidak valid.".into());
        }
        if matches!(name.to_ascii_lowercase().as_str(), "host" | "content-length" | "transfer-encoding") { return Err("Header transport tidak dapat ditimpa.".into()); }
    }
    #[cfg(windows)]
    { winhttp_request(method, &parsed, headers, body) }
    #[cfg(not(windows))]
    { let _ = (parsed, method, headers, body); Err("Transport HTTPS ini memerlukan Windows.".into()) }
}

#[cfg(windows)]
fn winhttp_request(method: &str, url: &HttpsUrl, headers: &[(&str, &str)], body: &[u8]) -> Result<Vec<u8>, String> {
    use std::{ffi::c_void, ptr};
    type Handle = *mut c_void;
    #[link(name = "winhttp", kind = "raw-dylib")]
    unsafe extern "system" {
        fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> Handle;
        fn WinHttpConnect(session: Handle, server: *const u16, port: u16, reserved: u32) -> Handle;
        fn WinHttpOpenRequest(connect: Handle, verb: *const u16, resource: *const u16, version: *const u16, referer: *const u16, accept: *const *const u16, flags: u32) -> Handle;
        fn WinHttpSetTimeouts(handle: Handle, resolve: i32, connect: i32, send: i32, receive: i32) -> i32;
        fn WinHttpSetOption(handle: Handle, option: u32, buffer: *const c_void, length: u32) -> i32;
        fn WinHttpSendRequest(request: Handle, headers: *const u16, length: u32, optional: *const c_void, optional_length: u32, total_length: u32, context: usize) -> i32;
        fn WinHttpReceiveResponse(request: Handle, reserved: *const c_void) -> i32;
        fn WinHttpQueryHeaders(request: Handle, info: u32, name: *const u16, buffer: *mut c_void, length: *mut u32, index: *mut u32) -> i32;
        fn WinHttpReadData(request: Handle, buffer: *mut c_void, length: u32, read: *mut u32) -> i32;
        fn WinHttpCloseHandle(handle: Handle) -> i32;
        fn WinHttpSetStatusCallback(handle:Handle,callback:Option<unsafe extern "system" fn(Handle,usize,u32,*mut c_void,u32)>,flags:u32,reserved:usize)->usize;
    }
    struct Owned(Handle);
    impl Drop for Owned { fn drop(&mut self) { unsafe { WinHttpCloseHandle(self.0); } } }
    fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
    fn error(operation: &str) -> String { format!("Koneksi HTTPS gagal saat {operation} (Windows {}).", std::io::Error::last_os_error().raw_os_error().unwrap_or(0)) }
    struct Context{events:mpsc::Sender<(u32,u32)>,body:Vec<u8>,headers:Vec<u16>,buffer:std::cell::UnsafeCell<Box<[u8;16384]>>}
    // Only the request worker accesses the buffer, and only after READ_COMPLETE.
    // WinHTTP retains it during a pending read; callback closure retains its allocation.
    unsafe impl Send for Context{}unsafe impl Sync for Context{}
    unsafe extern "system" fn callback(_:Handle,context:usize,status:u32,info:*mut c_void,length:u32){
        if context==0{return}let context_ptr=context as *const Context;
        if status==0x800 {unsafe{drop(Arc::from_raw(context_ptr));}return}
        let amount=if status==0x80000{length}else if status==0x200000&&!info.is_null(){unsafe{*((info as *const usize).add(1)as *const u32)}}else{0};
        let _=unsafe{&*context_ptr}.events.send((status,amount));
    }
    fn wait(receiver:&mpsc::Receiver<(u32,u32)>,expected:u32,began:Instant)->Result<u32,String>{loop{
        check_cancel()?;if began.elapsed()>Duration::from_secs(45){return Err("Request HTTPS melewati batas waktu total.".into())}
        match receiver.recv_timeout(Duration::from_millis(25)){Ok((0x200000,code))=>return Err(format!("Koneksi HTTPS gagal (Windows {code}).")),Ok((status,amount))if status==expected=>return Ok(amount),Ok(_)=>{},Err(mpsc::RecvTimeoutError::Timeout)=>{},Err(_)=>return Err("Koneksi HTTPS telah ditutup.".into())}
    }}
    unsafe {
        let agent = wide("mynotch/0.0.2");
        let session = Owned(WinHttpOpen(agent.as_ptr(), 4, ptr::null(), ptr::null(), 0x10000000));
        if session.0.is_null() { return Err(error("membuka sesi")); }
        if WinHttpSetTimeouts(session.0, 10000, 10000, 10000, 10000) == 0 { return Err(error("mengatur timeout")); }
        let host = wide(&url.host);
        let connection = Owned(WinHttpConnect(session.0, host.as_ptr(), url.port, 0));
        if connection.0.is_null() { return Err(error("menghubungkan server")); }
        let method = wide(method);
        let resource = wide(&url.resource);
        let request = Owned(WinHttpOpenRequest(connection.0, method.as_ptr(), resource.as_ptr(), ptr::null(), ptr::null(), ptr::null(), 0x00800000));
        if request.0.is_null() { return Err(error("membuat request")); }
        let mut header_text=String::from("Accept: application/json, text/calendar;q=0.9, */*;q=0.1\r\n");
        for(name,value)in headers{header_text.push_str(name);header_text.push_str(": ");header_text.push_str(value);header_text.push_str("\r\n");}
        let(tx,rx)=mpsc::channel();let context=Arc::new(Context{events:tx,body:body.to_vec(),headers:wide(&header_text),buffer:std::cell::UnsafeCell::new(Box::new([0;16384]))});
        // The callback owns one Arc until HANDLE_CLOSING, which is the final callback.
        // Request closure happens on this worker while async operations are pending.
        let raw=Arc::into_raw(context.clone())as usize;
        if WinHttpSetOption(request.0,45,&raw as *const _ as _,std::mem::size_of::<usize>()as u32)==0{drop(Arc::from_raw(raw as *const Context));return Err(error("mengatur konteks"));}
        if WinHttpSetStatusCallback(request.0,Some(callback),0x400000|0x20000|0x80000|0x200000|0x800,0)==usize::MAX{drop(Arc::from_raw(raw as *const Context));return Err(error("mengatur callback"));}
        // WINHTTP_OPTION_REDIRECT_POLICY / NEVER. No credential-bearing cross-origin redirect.
        let policy = 0_u32;
        if WinHttpSetOption(request.0, 88, &policy as *const _ as *const c_void, 4) == 0 { return Err(error("menonaktifkan redirect")); }
        // Disable cookies as well; the transport has no account cookie jar.
        let disabled = 1_u32;
        if WinHttpSetOption(request.0, 63, &disabled as *const _ as *const c_void, 4) == 0 { return Err(error("menonaktifkan cookie")); }
        let began = Instant::now();
        if WinHttpSendRequest(request.0, context.headers.as_ptr(), (context.headers.len() - 1) as u32, if context.body.is_empty() { ptr::null() } else { context.body.as_ptr() as _ }, context.body.len() as u32, context.body.len() as u32, raw) == 0 { return Err(error("mengirim request")); }
        wait(&rx,0x400000,began)?;
        if WinHttpReceiveResponse(request.0, ptr::null()) == 0 { return Err(error("menerima respons")); }
        wait(&rx,0x20000,began)?;
        let mut status = 0_u32; let mut size = 4_u32;
        if WinHttpQueryHeaders(request.0, 19 | 0x20000000, ptr::null(), &mut status as *mut _ as _, &mut size, ptr::null_mut()) == 0 { return Err(error("membaca status")); }
        if !(200..300).contains(&status) { return Err(format!("Layanan mengembalikan HTTP {status}.")); }
        let mut output = Vec::new();
        loop {
            if began.elapsed() > Duration::from_secs(45) { return Err("Request HTTPS melewati batas waktu total.".into()); }
            check_cancel()?;
            let buffer=&mut *context.buffer.get();
            if WinHttpReadData(request.0, buffer.as_mut_ptr() as _, buffer.len() as u32, ptr::null_mut()) == 0 { return Err(error("membaca respons")); }
            let read=wait(&rx,0x80000,began)?;
            if read == 0 { break; }
            if output.len() + read as usize > MAX_RESPONSE_BYTES { return Err("Respons layanan melebihi batas 8 MiB.".into()); }
            output.extend_from_slice(&buffer[..read as usize]);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "opt-in public HTTPS smoke test; no credentials or user data"]
    fn public_https_completion(){let body=get("https://www.microsoft.com/robots.txt").expect("public HTTPS completion");assert!(!body.is_empty());assert!(body.len()<MAX_RESPONSE_BYTES);}
    #[test]
    fn cancelled_work_never_opens_network_and_scope_restores(){let token=Cancellation::default();token.cancel();let result=with_cancel(token,||get("https://example.com"));assert_eq!(result.unwrap_err(),"Pekerjaan dibatalkan.");assert!(check_cancel().is_ok());}
    #[test]
    fn private_and_loopback_hosts_are_blocked_unless_allowlisted() {
        for host in ["127.0.0.1", "10.0.0.5", "172.16.0.9", "192.168.1.1", "169.254.1.2", "0.0.0.0", "255.255.255.255", "224.0.0.1", "240.0.0.1", "198.18.0.1", "192.0.2.1", "::1", "fe80::1", "fc00::1", "::ffff:127.0.0.1"] {
            assert!(host_is_blocked(host), "{host} must be blocked");
        }
        for host in ["example.com", "api.openai.com", "8.8.8.8", "1.1.1.1", "2001:4860:4860::8888"] {
            assert!(!host_is_blocked(host), "{host} must pass");
        }
        let allowed: HashSet<String> = ["127.0.0.1".to_string()].into_iter().collect();
        assert!(host_allowed("127.0.0.1", &allowed));
        assert!(!host_allowed("127.0.0.2", &allowed));
        let allowed_names: HashSet<String> = ["example.com".to_string()].into_iter().collect();
        assert!(host_allowed("EXAMPLE.com", &allowed_names));
    }
    #[test]
    #[cfg(windows)]
    fn cancellation_interrupts_pending_tls_without_waiting_for_timeout(){
        // The loopback fixture below needs the explicit allowlist: the transport now
        // blocks IP literals and private ranges unless allowlisted.
        set_allowed_hosts(&["127.0.0.1".to_string()]);
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();let port=listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();let(ready_tx,ready)=mpsc::channel();let(release_tx,release)=mpsc::channel();
        let server=std::thread::spawn(move||{let deadline=Instant::now()+Duration::from_secs(4);loop{match listener.accept(){Ok((socket,_))=>{let _=ready_tx.send(());let _=release.recv_timeout(Duration::from_secs(3));drop(socket);break},Err(e)if e.kind()==std::io::ErrorKind::WouldBlock&&Instant::now()<deadline=>std::thread::sleep(Duration::from_millis(5)),Err(_)=>break}}});
        let token=Cancellation::default();let worker_token=token.clone();let(tx,rx)=mpsc::channel();
        let worker=std::thread::spawn(move||{let _=tx.send(with_cancel(worker_token,||get(&format!("https://127.0.0.1:{port}/"))));});
        let accepted=ready.recv_timeout(Duration::from_secs(2));let began=Instant::now();token.cancel();let outcome=rx.recv_timeout(Duration::from_secs(1));let _=release_tx.send(());server.join().unwrap();worker.join().unwrap();
        set_allowed_hosts(&[]);
        assert!(accepted.is_ok(),"fixture connection was not established");assert_eq!(outcome.unwrap().unwrap_err(),"Pekerjaan dibatalkan.");assert!(began.elapsed()<Duration::from_secs(2));
    }
    #[test]
    fn only_https_without_userinfo_or_fragments() {
        assert!(parse_url("http://example.com").is_err());
        assert!(parse_url("https://user:key@example.com/feed").is_err());
        assert!(parse_url("https://example.com/a#secret").is_err());
        assert!(parse_url("https://example.com/\r\nInjected").is_err());
        assert_eq!(parse_url("https://example.com?x=1").unwrap().resource, "/?x=1");
        assert_eq!(parse_url("https://example.com:8443/a").unwrap().port, 8443);
    }
    #[test]
    fn invalid_headers_rejected_before_network() {
        assert!(request("GET", "https://example.com", &[("x-key", "a\r\nb")], &[]).unwrap_err().contains("Header"));
        assert!(request("DELETE", "https://example.com", &[], &[]).is_err());
    }
}
