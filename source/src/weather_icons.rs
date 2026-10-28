//! User-supplied MingCute weather artwork. PNG atlases are generated offline.
use windows::Win32::{Graphics::Imaging::*, System::Com::*};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Icon {
    ClearDay, ClearNight, MostlyDay, MostlyNight, PartlyDay, PartlyNight, Overcast,
    Fog, Drizzle, RainLight, Rain, RainHeavy, Downpour, Ice, SnowLight, Snow,
    SnowHeavy, Blizzard, Thunder, ThunderRain, Unknown,
}
impl Icon {
    pub fn select(code:u32,is_day:bool)->Self {
        use Icon::*;
        match code {
            0=>if is_day{ClearDay}else{ClearNight},
            1=>if is_day{MostlyDay}else{MostlyNight},
            2=>if is_day{PartlyDay}else{PartlyNight},
            3=>Overcast,45|48=>Fog,51=>Drizzle,53|61=>RainLight,
            55|63|80=>Rain,65|81=>RainHeavy,82=>Downpour,
            56|57|66|67|77=>Ice,71=>SnowLight,73=>Snow,75|85=>SnowHeavy,
            86=>Blizzard,95=>Thunder,96|99=>ThunderRain,_=>Unknown,
        }
    }
}
#[derive(Clone, Copy)]
pub enum Style { Gradient, Compact }
impl Style {
    pub fn tile(self)->u32 { match self { Self::Gradient=>96, Self::Compact=>40 } }
    pub fn bytes(self)->&'static [u8] { match self {
        Self::Gradient=>include_bytes!("../assets/weather/gradient.png"),
        Self::Compact=>include_bytes!("../assets/weather/compact.png"),
    } }
}
pub unsafe fn decode(style:Style)->windows::core::Result<IWICFormatConverter> {unsafe{
    let factory:IWICImagingFactory=CoCreateInstance(&CLSID_WICImagingFactory,None,CLSCTX_INPROC_SERVER)?;
    let stream=factory.CreateStream()?;stream.InitializeFromMemory(style.bytes())?;
    let decoder=factory.CreateDecoderFromStream(&stream,std::ptr::null(),WICDecodeMetadataCacheOnDemand)?;
    let frame=decoder.GetFrame(0)?;let(mut width,mut height)=(0,0);frame.GetSize(&mut width,&mut height)?;
    if (width,height)!=(style.tile()*7,style.tile()*3){return Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x80070057u32 as i32)))}
    let convert=factory.CreateFormatConverter()?;
    convert.Initialize(&frame,&GUID_WICPixelFormat32bppPBGRA,WICBitmapDitherTypeNone,None,0.,WICBitmapPaletteTypeCustom)?;
    Ok(convert)
}}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn wmo_precipitation_never_uses_fair_weather_and_daylight_is_preserved(){
        use Icon::*;
        for(code,day,want)in [(0,true,ClearDay),(0,false,ClearNight),(1,true,MostlyDay),(1,false,MostlyNight),(2,true,PartlyDay),(2,false,PartlyNight),(3,true,Overcast),(45,false,Fog),(48,true,Fog),(51,true,Drizzle),(53,false,RainLight),(55,true,Rain),(56,true,Ice),(57,false,Ice),(61,true,RainLight),(63,false,Rain),(65,true,RainHeavy),(66,true,Ice),(67,false,Ice),(71,true,SnowLight),(73,false,Snow),(75,true,SnowHeavy),(77,true,Ice),(80,true,Rain),(81,false,RainHeavy),(82,true,Downpour),(85,true,SnowHeavy),(86,false,Blizzard),(95,true,Thunder),(96,false,ThunderRain),(99,true,ThunderRain),(999,true,Unknown)]{
            assert_eq!(Icon::select(code,day),want,"WMO {code}, daylight {day}");
        }
    }
    #[test] fn real_atlases_decode_with_transparent_padding_and_visible_weather(){unsafe{
        CoInitializeEx(None,COINIT_APARTMENTTHREADED).ok().unwrap();
        struct Apartment;impl Drop for Apartment{fn drop(&mut self){unsafe{CoUninitialize();}}}let _apartment=Apartment;
        for style in [Style::Gradient,Style::Compact]{
            let converter=decode(style).unwrap();let(mut w,mut h)=(0,0);converter.GetSize(&mut w,&mut h).unwrap();
            let tile=style.tile();assert_eq!((w,h),(tile*7,tile*3));
            let mut pixels=vec![0;(w*h*4)as usize];converter.CopyPixels(std::ptr::null(),w*4,&mut pixels).unwrap();
            for cell in 0..21u32{
                let(left,top)=((cell%7)*tile,(cell/7)*tile);let mut visible=0;
                for y in 0..tile{for x in 0..tile{let i=(((top+y)*w+left+x)*4)as usize;let alpha=pixels[i+3];
                    if x==0||y==0||x==tile-1||y==tile-1{assert_eq!(alpha,0,"sheet background must be removed");}
                    assert!(pixels[i..i+3].iter().all(|c|*c<=alpha),"WIC must premultiply alpha");
                    if alpha>100{visible+=1;}
                }}
                assert!(visible>tile*tile/10,"empty weather cell {cell}");
            }
        }
    }}
}
