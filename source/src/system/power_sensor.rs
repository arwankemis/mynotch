//! Read-only WMI sensor discovery. No driver loading, elevation, or polling subprocesses.
use super::{PowerSnapshot,SourceKind,Quality};
use std::{sync::{Arc,Mutex,mpsc},thread,time::{Duration,Instant}};
use windows::{core::{BSTR,PCWSTR,Result},Win32::System::{Com::*,Wmi::*,Variant::VARIANT}};

mod bundled;
enum Control{Active(bool),Setup,Stop}
fn fresh_snapshot(mut snapshot:PowerSnapshot,age:Duration)->PowerSnapshot{
    if age>Duration::from_secs(6){snapshot.cpu_setup_required=false;snapshot.active_power_w=None;snapshot.cpu_watts=None;snapshot.gpu_watts=None;snapshot.total_dc_watts=None;snapshot.source_kind=SourceKind::Unavailable;snapshot.quality=Quality::Stale;snapshot.provider_note=Some("Sensor belum memberikan data baru; pencatatan energi dijeda.".into());}snapshot
}
pub struct PowerSensors{state:Arc<Mutex<(PowerSnapshot,Instant)>>,stop:mpsc::Sender<Control>,stopped:mpsc::Receiver<()>,active:std::sync::atomic::AtomicBool}
impl PowerSensors{
    pub fn start()->Self{
        let state=Arc::new(Mutex::new((PowerSnapshot{provider_note:Some("Mendeteksi perangkat dan sensor daya…".into()),..Default::default()},Instant::now())));
        let shared=state.clone();let(stop,rx)=mpsc::channel();let(stopped_tx,stopped)=mpsc::channel();
        let _=thread::Builder::new().name("notch-power-sensors".into()).spawn(move||unsafe{
            struct Finished(mpsc::Sender<()>);impl Drop for Finished{fn drop(&mut self){let _=self.0.send(());}}let _finished=Finished(stopped_tx);
            let helper_path=std::env::current_exe().ok().and_then(|p|p.parent().map(|d|d.join("power-provider/mynotch-power.exe")));
            let has_helper=helper_path.as_ref().is_some_and(|p|p.is_file());
            let com_ready=!has_helper&&CoInitializeEx(None,COINIT_MULTITHREADED).is_ok();
            struct Apartment(bool);impl Drop for Apartment{fn drop(&mut self){if self.0{unsafe{CoUninitialize()}}}}let _apartment=Apartment(com_ready);
            let locator:Option<IWbemLocator>=if com_ready{CoCreateInstance(&WbemLocator,None,CLSCTX_INPROC_SERVER).ok()}else{None};
            let system=locator.as_ref().and_then(|l|connect(l,r"ROOT\CIMV2").ok());
            let cpu=system.as_ref().and_then(|s|names(s,"Win32_Processor").into_iter().next());
            let gpu=system.as_ref().and_then(|s|names(s,"Win32_VideoController").into_iter().find(|n|!n.contains("Microsoft Basic")));
            let mut provider:Option<(String,IWbemServices)>=None;let mut retries=0;
            let mut helper:Option<bundled::Session>=None;let mut helper_frame:Option<(bundled::Frame,Instant)>=None;
            let mut active=true;
            loop{
                if !active{helper=None;helper_frame=None;retries=0;match rx.recv(){Ok(Control::Active(v))=>{active=v;continue},Ok(Control::Setup)=>continue,_=>break}}
                let acquired=Instant::now();
                if helper.is_none()&&retries%15==0{if let Some(path)=helper_path.as_ref().filter(|p|p.is_file()){helper=bundled::Session::start(path).ok();}}
                if let Some(session)=helper.as_mut(){if let Some(frame)=session.latest(){helper_frame=Some(frame);}if session.exited(){helper=None;}}
                if helper.is_none()&&provider.is_none()&&retries%15==0{for ns in [r"ROOT\LibreHardwareMonitor",r"ROOT\OpenHardwareMonitor"]{
                    if let Some(locator)=&locator{if let Ok(service)=connect(locator,ns){provider=Some((ns.into(),service));break}}
                }}
                retries+=1;
                let mut snapshot=PowerSnapshot{cpu_name:cpu.clone(),gpu_name:gpu.clone(),provider_note:Some(if has_helper{"Pembaca sensor bawaan belum memberikan data baru; mencoba menghubungkan kembali."}else{"Pembaca sensor bawaan tidak ditemukan; fallback sensor WMI belum tersedia."}.into()),..Default::default()};
                if let Some((frame,_))=helper_frame.as_ref().filter(|(_,at)|at.elapsed()<=Duration::from_secs(6)){frame.apply(&mut snapshot);}
                else if let Some((name,service))=&provider{
                    let mut readings=Vec::new();
                    if let Ok(objects)=query(service,"SELECT Name,Identifier,Parent,Value FROM Sensor WHERE SensorType='Power'"){
                        for o in objects{
                            if let Some(value)=number(&o,"Value"){readings.push(SensorReading{name:string(&o,"Name").unwrap_or_default(),identifier:string(&o,"Identifier").unwrap_or_default(),parent:string(&o,"Parent").unwrap_or_default(),watts:value});}
                        }
                        apply_readings(&mut snapshot,&readings,name);
                    }else{snapshot.provider_note=Some("Provider sensor tidak merespons; tidak ada pengukuran daya baru.".into());provider=None;}
                }
                if let Ok(mut state)=shared.lock(){*state=(snapshot,acquired);}
                match rx.recv_timeout(Duration::from_secs(2)){Ok(Control::Active(v))=>active=v,Ok(Control::Setup)=>{if let Some(session)=helper.as_mut(){session.request_admin();}},Ok(Control::Stop)|Err(mpsc::RecvTimeoutError::Disconnected)=>break,Err(mpsc::RecvTimeoutError::Timeout)=>{}}
            }
        });
        Self{state,stop,stopped,active:std::sync::atomic::AtomicBool::new(true)}
    }
    pub fn request_admin(&self){let _=self.stop.send(Control::Setup);}
    pub fn set_active(&self,enabled:bool){
        if self.active.swap(enabled,std::sync::atomic::Ordering::AcqRel)!=enabled{
            if enabled{if let Ok(mut s)=self.state.lock(){let s=&mut s.0;s.cpu_setup_required=false;s.active_power_w=None;s.cpu_watts=None;s.gpu_watts=None;s.quality=Quality::Stale;s.provider_note=Some("Memperbarui sensor…".into());}}
            let _=self.stop.send(Control::Active(enabled));
        }
    }
    pub fn snapshot(&self)->PowerSnapshot{self.state.lock().map(|s|fresh_snapshot(s.0.clone(),s.1.elapsed())).unwrap_or_default()}
}
impl Drop for PowerSensors{fn drop(&mut self){let _=self.stop.send(Control::Stop);let _=self.stopped.recv_timeout(Duration::from_secs(3));}}

unsafe fn connect(locator:&IWbemLocator,namespace:&str)->Result<IWbemServices>{unsafe{
    let empty=BSTR::new();let service=locator.ConnectServer(&BSTR::from(namespace),&empty,&empty,&empty,128,&empty,None)?;
    CoSetProxyBlanket(&service,10,0,PCWSTR::null(),RPC_C_AUTHN_LEVEL_CALL,RPC_C_IMP_LEVEL_IMPERSONATE,None,EOAC_NONE)?;
    Ok(service)
}}
unsafe fn query(service:&IWbemServices,sql:&str)->Result<Vec<IWbemClassObject>>{unsafe{
    let iter=service.ExecQuery(&BSTR::from("WQL"),&BSTR::from(sql),WBEM_FLAG_FORWARD_ONLY|WBEM_FLAG_RETURN_IMMEDIATELY,None)?;
    let mut objects=Vec::new();for _ in 0..64{let mut row=[None];let mut count=0;iter.Next(500,&mut row,&mut count).ok()?;if count==0{break}if let Some(o)=row[0].take(){objects.push(o)}}Ok(objects)
}}
unsafe fn value(o:&IWbemClassObject,name:&str)->Option<VARIANT>{unsafe{let key:Vec<u16>=name.encode_utf16().chain(Some(0)).collect();let mut v=VARIANT::default();o.Get(PCWSTR(key.as_ptr()),0,&mut v,None,None).ok()?;Some(v)}}
unsafe fn string(o:&IWbemClassObject,name:&str)->Option<String>{unsafe{BSTR::try_from(&value(o,name)?).ok().map(|s|s.to_string().trim().to_owned())}}
unsafe fn number(o:&IWbemClassObject,name:&str)->Option<f32>{unsafe{f64::try_from(&value(o,name)?).ok().map(|n|n as f32)}}
unsafe fn names(s:&IWbemServices,class:&str)->Vec<String>{unsafe{query(s,&format!("SELECT Name FROM {class}")).unwrap_or_default().iter().filter_map(|o|string(o,"Name")).collect()}}

#[derive(Clone)]pub struct SensorReading{pub name:String,pub identifier:String,pub parent:String,pub watts:f32}
pub fn apply_readings(snapshot:&mut PowerSnapshot,readings:&[SensorReading],provider:&str){
    fn pick(readings:&[SensorReading],gpu:bool)->Option<f32>{
        readings.iter().filter(|r|r.watts.is_finite()&&r.watts>=0.&&r.watts<=3000.)
            .filter(|r|{let id=format!("{} {}",r.identifier,r.parent).to_ascii_lowercase();if gpu{id.contains("gpu")}else{id.contains("cpu")}})
            .max_by_key(|r|{let name=r.name.to_ascii_lowercase();if name.contains("total")||name.contains("board"){3}else if name.contains("package"){2}else if name.contains("core"){1}else{0}}).map(|r|r.watts)
    }
    snapshot.cpu_watts=pick(readings,false);snapshot.gpu_watts=pick(readings,true);
    if snapshot.cpu_watts.is_some()||snapshot.gpu_watts.is_some(){
        snapshot.active_power_w=Some(snapshot.cpu_watts.unwrap_or(0.)+snapshot.gpu_watts.unwrap_or(0.));snapshot.total_dc_watts=snapshot.active_power_w;
        snapshot.source_kind=SourceKind::Derived;snapshot.quality=Quality::Partial;
        snapshot.provider_note=Some(format!("{provider} · subtotal sensor CPU/GPU, bukan daya total PC/stopkontak"));
    }else{snapshot.provider_note=Some(format!("{provider} terdeteksi, tetapi sensor watt CPU/GPU tidak tersedia."));}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn stalled_sensor_snapshot_cannot_keep_integrating_cached_watts(){
        let s=PowerSnapshot{active_power_w:Some(150.),cpu_watts:Some(50.),gpu_watts:Some(100.),total_dc_watts:Some(150.),..Default::default()};
        assert_eq!(fresh_snapshot(s.clone(),Duration::from_secs(2)).active_power_w,Some(150.));
        let stale=fresh_snapshot(s,Duration::from_secs(7));
        assert_eq!(stale.active_power_w,None);assert_eq!(stale.cpu_watts,None);assert_eq!(stale.gpu_watts,None);assert_eq!(stale.total_dc_watts,None);assert_eq!(stale.quality,Quality::Stale);
    }
    #[test]fn sensors_choose_package_and_board_without_double_counting_or_inventing_wall_power(){
        let r=|name:&str,id:&str,w|SensorReading{name:name.into(),identifier:id.into(),parent:id.into(),watts:w};
        let mut s=PowerSnapshot::default();apply_readings(&mut s,&[r("CPU Package","/amdcpu/0",65.),r("CPU Cores","/amdcpu/0",40.),r("GPU Total Board","/gpu-amd/0",120.),r("GPU Core","/gpu-amd/0",90.)],"test");
        assert_eq!(s.active_power_w,Some(185.));assert_eq!(s.total_ac_watts,None);assert_eq!(s.current_a,None);assert_eq!(s.voltage_v,None);
    }
    #[test]fn absent_or_invalid_sensors_remain_unavailable(){let mut s=PowerSnapshot::default();apply_readings(&mut s,&[],"test");assert_eq!(s.active_power_w,None);assert!(s.provider_note.unwrap().contains("tidak tersedia"));}
}
