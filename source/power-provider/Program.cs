using LibreHardwareMonitor.Hardware;
using Microsoft.Win32;
using System.Diagnostics;
using System.IO.Pipes;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace Mynotch.Power;
internal record Sample(string Name,float? Watts);
internal record Frame(int Schema,string Backend,string? CpuName,string? GpuName,float? CpuWatts,float? GpuWatts,bool DriverInstalled,bool Elevated,bool SetupRequired,string Status);
internal static class Program{
 static readonly JsonSerializerOptions Json=new(){PropertyNamingPolicy=JsonNamingPolicy.SnakeCaseLower};
 const string DriverSha="A3A46226C5E2824F4CDD42BE0EECBABFC672C86F7889710F5AB1E6AD385B47A0";
 static readonly CancellationTokenSource Stop=new();
 static Frame? AdminFrame;static long AdminAt;static Task? AdminTask;static string? SetupError;
 static bool DriverReady{get{try{using var registry=RegistryKey.OpenBaseKey(RegistryHive.LocalMachine,RegistryView.Registry64);using var key=registry.OpenSubKey(@"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO");return Version.TryParse(key?.GetValue("DisplayVersion")as string,out var version)&&version>=new Version(2,1,0,0);}catch{return false;}}}
 static bool IsAdmin()=>new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator);
 static bool Valid(float? x)=>x is >0 and <=3000&&float.IsFinite(x.Value);
 internal static float? Pick(IEnumerable<Sample> samples,bool gpu,bool updated=true){
  if(!updated)return null;
  int Rank(string name){name=name.ToLowerInvariant();if(name.Contains("core")||name.Contains("memory"))return 0;if(name.Contains("board"))return 4;if(name.Contains("total"))return 3;if(name.Contains("package"))return 2;return gpu&&name=="gpu power"?1:0;}
  return samples.Where(s=>Valid(s.Watts)&&Rank(s.Name)>0).OrderByDescending(s=>Rank(s.Name)).Select(s=>s.Watts).FirstOrDefault();
 }
 static IEnumerable<IHardware> Walk(IHardware h){yield return h;foreach(var child in h.SubHardware)foreach(var v in Walk(child))yield return v;}
 static Frame Read(Computer computer,bool elevated){
  var updated=new HashSet<IHardware>();
  foreach(var hw in computer.Hardware)foreach(var h in Walk(hw)){try{h.Update();updated.Add(h);}catch{}}
  var all=computer.Hardware.SelectMany(Walk).ToArray();
  var cpu=all.FirstOrDefault(h=>h.HardwareType==HardwareType.Cpu);
  var gpu=all.FirstOrDefault(h=>h.HardwareType is HardwareType.GpuAmd or HardwareType.GpuNvidia or HardwareType.GpuIntel);
  static IEnumerable<Sample> Power(IHardware? h)=>h?.Sensors.Where(s=>s.SensorType==SensorType.Power).Select(s=>new Sample(s.Name,s.Value))??[];
  var cw=Pick(Power(cpu),false,cpu is not null&&updated.Contains(cpu));var gw=Pick(Power(gpu),true,gpu is not null&&updated.Contains(gpu));
  return new(1,"librehardwaremonitor-0.9.6",cpu?.Name,gpu?.Name,cw,gw,DriverReady,elevated,cw is null&&!elevated,cw is not null&&gw is not null?"ready":cw is not null||gw is not null?"partial":"unavailable");
 }
 static async Task ReadCommands(){
  try{while(!Stop.IsCancellationRequested){var line=await Console.In.ReadLineAsync(Stop.Token);if(line is null)break;if(line=="cpu-admin"&&!Stop.IsCancellationRequested&&(AdminTask is null||AdminTask.IsCompleted)){SetupError=null;Volatile.Write(ref AdminFrame,null);Interlocked.Exchange(ref AdminAt,0);AdminTask=Task.Run(ReadElevated);}}}catch(OperationCanceledException){}finally{Stop.Cancel();}
 }
 static async Task ReadElevated(){
  if(Stop.IsCancellationRequested)return;
  var name="mynotch-power-"+Guid.NewGuid().ToString("N");
  using var pipe=new NamedPipeServerStream(name,PipeDirection.In,1,PipeTransmissionMode.Byte,PipeOptions.Asynchronous|PipeOptions.CurrentUserOnly);
  using var connection=CancellationTokenSource.CreateLinkedTokenSource(Stop.Token);connection.CancelAfter(TimeSpan.FromMinutes(3));
  try{
   using var process=Process.Start(new ProcessStartInfo(Environment.ProcessPath!,"--admin-pipe="+name){UseShellExecute=true,Verb="runas",WindowStyle=ProcessWindowStyle.Hidden,WorkingDirectory=AppContext.BaseDirectory});
   var connected=pipe.WaitForConnectionAsync(connection.Token);
   if(process is not null){var ended=process.WaitForExitAsync(connection.Token);if(await Task.WhenAny(connected,ended)!=connected&&!pipe.IsConnected)throw new IOException("CPU setup exited before connection");}
   await connected;connection.CancelAfter(Timeout.InfiniteTimeSpan);
   using var reader=new StreamReader(pipe);
   while(!Stop.IsCancellationRequested){var line=await reader.ReadLineAsync(Stop.Token);if(line is null)break;if(line.Length>8192)break;var f=JsonSerializer.Deserialize<Frame>(line,Json);if(f is null||f.Schema!=1||!f.Elevated)continue;if(f.CpuWatts is not null&&!Valid(f.CpuWatts))continue;Volatile.Write(ref AdminFrame,f);Interlocked.Exchange(ref AdminAt,Environment.TickCount64);}
  }catch(System.ComponentModel.Win32Exception ex)when(ex.NativeErrorCode==1223){SetupError="admin_cancelled";}catch(OperationCanceledException){}catch{SetupError="admin_unavailable";}
 }
 static Task Heartbeat(StreamWriter output)=>output.WriteLineAsync(JsonSerializer.Serialize(new Frame(1,"librehardwaremonitor-0.9.6",null,null,null,null,false,true,false,"driver_setup_pending"),Json));
 static async Task<bool> InLiveSession(StreamWriter output,Func<Task<bool>> action){await Heartbeat(output);return await action();}
 static async Task<bool> EnsureDriver(StreamWriter output){
  if(DriverReady)return true;
  var setup=Path.Combine(AppContext.BaseDirectory,"driver","PawnIO_setup.exe");
  if(!File.Exists(setup)||Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(setup)))!=DriverSha)return false;
  using var install=Process.Start(new ProcessStartInfo(setup){UseShellExecute=true,WorkingDirectory=Path.GetDirectoryName(setup)!});
  if(install is null)return false;
  try{
   var deadline=Environment.TickCount64+180000;
   while(!install.HasExited&&Environment.TickCount64<deadline){await Heartbeat(output);await Task.Delay(500);}
   return install.HasExited&&install.ExitCode==0&&DriverReady;
  }finally{
   // Request graceful cancellation if the parent closes; never kill a driver installer mid-write.
   if(!install.HasExited){try{install.CloseMainWindow();}catch{}}
  }
 }
 static async Task Admin(string pipeName){
  if(!IsAdmin()||!Regex.IsMatch(pipeName,"^mynotch-power-[a-f0-9]{32}$"))return;
  // Connect and verify a live session before accessing or installing the driver.
  using var pipe=new NamedPipeClientStream(".",pipeName,PipeDirection.Out,PipeOptions.Asynchronous);await pipe.ConnectAsync(15000);
  using var output=new StreamWriter(pipe){AutoFlush=true};
  if(!DriverReady&&!await InLiveSession(output,()=>EnsureDriver(output))){await output.WriteLineAsync(JsonSerializer.Serialize(new Frame(1,"librehardwaremonitor-0.9.6",null,null,null,null,DriverReady,true,true,"driver_setup_cancelled"),Json));return;}
  var computer=new Computer{IsCpuEnabled=true};
  try{computer.Open();while(pipe.IsConnected){await output.WriteLineAsync(JsonSerializer.Serialize(Read(computer,true),Json));await Task.Delay(2000);}}catch(IOException){}finally{computer.Close();}
 }
 static async Task Test(){
  void Check(bool value,string message){if(!value)throw new Exception(message);}
  Check(Pick([new("Package",65),new("Core #1",90)],false)==65,"CPU package precedence");
  Check(Pick([new("GPU Package",100),new("GPU Total Board",130)],true)==130,"GPU board precedence");
  Check(Pick([new("GPU Package",50)],true,false)is null,"failed hardware update cannot reuse cached power");
  Check(Pick([new("Package",0),new("Package",float.NaN),new("Package",-4)],false)is null,"invalid/zero is missing");
  var f=new Frame(1,"test","CPU","GPU",null,50,false,false,true,"partial");var round=JsonSerializer.Deserialize<Frame>(JsonSerializer.Serialize(f,Json),Json);Check(round==f,"wire roundtrip");
  var name="mynotch-power-"+Guid.NewGuid().ToString("N");
  using var server=new NamedPipeServerStream(name,PipeDirection.In,1,PipeTransmissionMode.Byte,PipeOptions.Asynchronous|PipeOptions.CurrentUserOnly);
  using var client=new NamedPipeClientStream(".",name,PipeDirection.Out,PipeOptions.Asynchronous);
  var accepted=server.WaitForConnectionAsync();await client.ConnectAsync(1000);await accepted;server.Dispose();
  var writer=new StreamWriter(client){AutoFlush=true};var setupCalls=0;
  try{await InLiveSession(writer,()=>{setupCalls++;return Task.FromResult(true);});}catch(IOException){}finally{try{writer.Dispose();}catch(IOException){}}
  Check(setupCalls==0,"closed parent must prevent driver setup");
  Console.WriteLine("6 provider contract checks passed");
 }
 static async Task<int> Main(string[] args){
  if(args.Contains("--self-test")){await Test();return 0;}
  var admin=args.FirstOrDefault(a=>a.StartsWith("--admin-pipe="));
  if(admin is not null){try{await Admin(admin[13..]);}catch{}return 0;}
  if(!args.Contains("--once")&&!args.Contains("--stream"))return 2;
  var once=args.Contains("--once");var elevated=IsAdmin();var computer=new Computer{IsCpuEnabled=true,IsGpuEnabled=true};
  if(!once)_=Task.Run(ReadCommands);
  try{
   computer.Open();Read(computer,elevated);await Task.Delay(500);
   while(!Stop.IsCancellationRequested){
    var f=Read(computer,elevated);var af=Volatile.Read(ref AdminFrame);var fresh=af is not null&&Environment.TickCount64-Interlocked.Read(ref AdminAt)<=6000;
    if(fresh){f=f with{CpuName=af!.CpuName??f.CpuName,CpuWatts=af.CpuWatts,DriverInstalled=af.DriverInstalled,SetupRequired=af.SetupRequired};}
    if(f.CpuWatts is not null&&f.GpuWatts is not null)f=f with{Status="ready"};
    else if(fresh&&af!.Status is "driver_setup_pending" or "driver_setup_cancelled")f=f with{Status=af.Status};
    else if(AdminTask is {IsCompleted:false}&&!fresh)f=f with{SetupRequired=false,Status="cpu_setup_pending"};
    else if(SetupError is not null)f=f with{Status=SetupError};
    else if(f.SetupRequired)f=f with{Status=f.DriverInstalled?"cpu_admin_required":"cpu_driver_setup_required"};
    else if(f.CpuWatts is null)f=f with{Status="cpu_unavailable"};
    Console.WriteLine(JsonSerializer.Serialize(f,Json));if(once)break;
    await Task.Delay(2000,Stop.Token);
   }
  }catch(OperationCanceledException){}catch(Exception ex){Console.WriteLine(JsonSerializer.Serialize(new{schema=1,backend="librehardwaremonitor-0.9.6",status="provider_error",error=ex.GetType().Name},Json));return 1;}
  finally{Stop.Cancel();computer.Close();if(AdminTask is not null)await Task.WhenAny(AdminTask,Task.Delay(2500));}
  return 0;
 }
}
