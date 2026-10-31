use std::{io::{BufRead,BufReader,Read,Write},path::Path,process::{Child,Command,Stdio},sync::mpsc,thread,time::Instant};
use std::os::windows::process::CommandExt;
use serde::Deserialize;
use super::super::{PowerSnapshot,Quality,SourceKind};

#[derive(Clone,Debug,Deserialize)]
pub struct Frame{
 pub schema:u32,pub backend:String,pub cpu_name:Option<String>,pub gpu_name:Option<String>,pub cpu_watts:Option<f32>,pub gpu_watts:Option<f32>,
 #[serde(default)]pub driver_installed:bool,#[serde(default)]pub setup_required:bool,pub status:String,
}
impl Frame{
 pub fn decode(bytes:&[u8])->Option<Self>{
  if bytes.len()>32768{return None}let f:Self=serde_json::from_slice(bytes).ok()?;
  if f.schema!=1||f.backend!="librehardwaremonitor-0.9.6"{return None}
  if [&f.cpu_name,&f.gpu_name].into_iter().flatten().any(|s|s.len()>512)||f.status.len()>80{return None}
  if [f.cpu_watts,f.gpu_watts].into_iter().flatten().any(|w|!w.is_finite()||w<=0.||w>3000.){return None}Some(f)
 }
 pub fn apply(&self,s:&mut PowerSnapshot){
  if self.cpu_name.is_some(){s.cpu_name=self.cpu_name.clone();}if self.gpu_name.is_some(){s.gpu_name=self.gpu_name.clone();}
  s.cpu_watts=self.cpu_watts;s.gpu_watts=self.gpu_watts;s.cpu_setup_required=self.setup_required;
  if self.cpu_watts.is_some()||self.gpu_watts.is_some(){s.active_power_w=Some(self.cpu_watts.unwrap_or(0.)+self.gpu_watts.unwrap_or(0.));s.total_dc_watts=s.active_power_w;s.source_kind=SourceKind::Derived;s.quality=Quality::Partial;}
  s.provider_note=Some(match self.status.as_str(){
   "ready"=>"Sensor bawaan · subtotal CPU/GPU, bukan listrik seluruh PC",
   "cpu_admin_required"=>"Pembacaan CPU memerlukan izin administrator",
   "cpu_driver_setup_required"=>"Siapkan sensor CPU untuk memasang driver pembaca yang disertakan",
   "cpu_setup_pending"=>"Menunggu izin Windows untuk membaca sensor CPU",
   "driver_setup_pending"=>"Penyiapan driver sensor CPU sedang berlangsung; ikuti jendela setup Windows",
   "driver_setup_cancelled"=>"Penyiapan CPU belum selesai; sensor yang tersedia tetap berjalan",
   "admin_cancelled"=>"Izin CPU dibatalkan; sensor yang tersedia tetap berjalan",
   "admin_unavailable"=>"Sesi sensor CPU belum tersambung; coba penyiapan sensor kembali",
   "cpu_unavailable"=>"Sensor CPU belum memberikan watt valid; data sensor yang tersedia ditampilkan",
   "provider_error"=>"Pembaca sensor bawaan gagal; coba aktifkan ulang Mod Daya",
   _=>if self.driver_installed{"Sensor bawaan berjalan; sebagian sensor watt belum tersedia"}else{"Pembaca sensor bawaan siap; sensor CPU membutuhkan penyiapan driver"},
  }.into());
 }
}
pub struct Session{child:Child,rx:mpsc::Receiver<(Frame,Instant)>,reader:Option<thread::JoinHandle<()>>}
impl Session{
 pub fn start(path:&Path)->std::io::Result<Self>{
  let mut child=Command::new(path).arg("--stream").current_dir(path.parent().unwrap_or(Path::new("."))).creation_flags(0x08000000).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
  let stdout=child.stdout.take().unwrap();let(tx,rx)=mpsc::sync_channel(2);
  let reader=thread::spawn(move||{let mut r=BufReader::new(stdout);loop{let mut line=Vec::new();match r.by_ref().take(32769).read_until(b'\n',&mut line){Ok(0)|Err(_)=>break,_=>{}}if line.len()>32768{break}if let Some(frame)=Frame::decode(&line){match tx.try_send((frame,Instant::now())){Ok(())|Err(mpsc::TrySendError::Full(_))=>{},Err(mpsc::TrySendError::Disconnected(_))=>break}}}});
  Ok(Self{child,rx,reader:Some(reader)})
 }
 pub fn latest(&self)->Option<(Frame,Instant)>{let mut f=None;while let Ok(v)=self.rx.try_recv(){f=Some(v)}f}
 pub fn exited(&mut self)->bool{self.child.try_wait().ok().flatten().is_some()}
 pub fn request_admin(&mut self){if let Some(input)=self.child.stdin.as_mut(){let _=input.write_all(b"cpu-admin\n");let _=input.flush();}}
}
impl Drop for Session{fn drop(&mut self){
 self.child.stdin.take();let _=self.child.kill();let _=self.child.wait();if let Some(reader)=self.reader.take(){let _=reader.join();}
}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn bundled_sensor_frame_maps_components_without_inventing_ac(){
  let f=Frame::decode(br#"{"schema":1,"backend":"librehardwaremonitor-0.9.6","cpu_name":"CPU","gpu_name":"GPU","cpu_watts":null,"gpu_watts":50,"setup_required":true,"driver_installed":true,"status":"cpu_admin_required"}"#).unwrap();
  let mut s=PowerSnapshot::default();f.apply(&mut s);assert_eq!(s.active_power_w,Some(50.));assert_eq!(s.cpu_watts,None);assert!(s.cpu_setup_required);assert_eq!(s.total_ac_watts,None);assert_eq!(s.voltage_v,None);
 }
 #[test]fn malformed_or_out_of_bounds_sensor_frames_are_rejected(){
  for bytes in [br#"{"schema":2,"backend":"wrong","status":"ready"}"#.as_slice(),br#"{"schema":1,"backend":"librehardwaremonitor-0.9.6","status":"ready","cpu_watts":-1}"#.as_slice()]{assert!(Frame::decode(bytes).is_none());}
  assert!(Frame::decode(&vec![b' ';32769]).is_none());
 }
}
