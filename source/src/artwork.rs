//! Bounded WIC artwork decode on a worker; only compact premultiplied pixels reach the UI.
use std::hash::{Hash,Hasher};
use windows::Win32::{Graphics::Imaging::*,System::Com::*};

#[derive(Clone)]
pub struct Artwork{pub pixels:Vec<u8>,pub width:u32,pub height:u32,pub accent:u32}
pub fn fingerprint(bytes:&[u8])->u64{if bytes.is_empty(){return 0}let mut h=std::collections::hash_map::DefaultHasher::new();bytes.hash(&mut h);h.finish()}

pub fn decode(bytes:&[u8])->Result<Artwork,String>{
    crate::net::check_cancel()?;
    if bytes.is_empty()||bytes.len()>4*1024*1024{return Err("Ukuran cover tidak didukung.".into())}
    unsafe{
        CoInitializeEx(None,COINIT_MULTITHREADED).ok().map_err(|e|e.to_string())?;
        struct Apartment;impl Drop for Apartment{fn drop(&mut self){unsafe{CoUninitialize();}}}let _apartment=Apartment;
        let result=(||->windows::core::Result<Artwork>{
            let factory:IWICImagingFactory=CoCreateInstance(&CLSID_WICImagingFactory,None,CLSCTX_INPROC_SERVER)?;
            let stream=factory.CreateStream()?;stream.InitializeFromMemory(bytes)?;
            let decoder=factory.CreateDecoderFromStream(&stream,std::ptr::null(),WICDecodeMetadataCacheOnDemand)?;
            let frame=decoder.GetFrame(0)?;let(mut width,mut height)=(0,0);frame.GetSize(&mut width,&mut height)?;
            if width==0||height==0||width>16384||height>16384||u64::from(width)*u64::from(height)>8_388_608{return Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x80070057u32 as i32)))}
            let side=width.min(height);let clip=factory.CreateBitmapClipper()?;
            clip.Initialize(&frame,&WICRect{X:((width-side)/2)as i32,Y:((height-side)/2)as i32,Width:side as i32,Height:side as i32})?;
            let scale=factory.CreateBitmapScaler()?;scale.Initialize(&clip,256,256,WICBitmapInterpolationModeFant)?;
            let convert=factory.CreateFormatConverter()?;convert.Initialize(&scale,&GUID_WICPixelFormat32bppPBGRA,WICBitmapDitherTypeNone,None,0.,WICBitmapPaletteTypeCustom)?;
            let mut pixels=vec![0;256*256*4];convert.CopyPixels(std::ptr::null(),256*4,&mut pixels)?;
            let mut rgb=[0u64;3];let mut count=0u64;let mut vivid=[0u64;3];let mut vivid_count=0u64;
            for pixel in pixels.chunks_exact(4){if pixel[3]>200{rgb[0]+=pixel[2]as u64;rgb[1]+=pixel[1]as u64;rgb[2]+=pixel[0]as u64;count+=1;
                let saturation=(*pixel[..3].iter().max().unwrap()as u64).saturating_sub(*pixel[..3].iter().min().unwrap()as u64);if saturation>35{let weight=saturation*saturation;vivid[0]+=pixel[2]as u64*weight;vivid[1]+=pixel[1]as u64*weight;vivid[2]+=pixel[0]as u64*weight;vivid_count+=weight;}
            }}
            if vivid_count>0{rgb=vivid;count=vivid_count;}
            let accent=if count==0{0x82e0c9}else{let r=(rgb[0]/count).clamp(35,235)as u32;let g=(rgb[1]/count).clamp(35,235)as u32;let b=(rgb[2]/count).clamp(35,235)as u32;(r<<16)|(g<<8)|b};
            // Bake antialiased rounded corners once instead of creating a mask each frame.
            for y in 0..256usize{for x in 0..256usize{let dx=(x as f32+0.5-128.).abs()-108.;let dy=(y as f32+0.5-128.).abs()-108.;if dx>0.&&dy>0.{let coverage=(20.5-dx.hypot(dy)).clamp(0.,1.);let i=(y*256+x)*4;for channel in &mut pixels[i..i+4]{*channel=(*channel as f32*coverage).round()as u8;}}}}
            Ok(Artwork{pixels,width:256,height:256,accent})
        })();
        crate::net::check_cancel()?;result.map_err(|_|"Cover tidak dapat dibaca.".into())
    }
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn invalid_and_empty_artwork_are_rejected(){assert!(decode(&[]).is_err());assert!(decode(b"not an image").is_err());assert_eq!(fingerprint(&[]),0);}
    #[test]fn wic_decodes_centercrops_and_rounds_fixture(){
        let mut bmp=vec![0u8;70];bmp[0..2].copy_from_slice(b"BM");bmp[2..6].copy_from_slice(&70u32.to_le_bytes());bmp[10..14].copy_from_slice(&54u32.to_le_bytes());bmp[14..18].copy_from_slice(&40u32.to_le_bytes());bmp[18..22].copy_from_slice(&2i32.to_le_bytes());bmp[22..26].copy_from_slice(&2i32.to_le_bytes());bmp[26..28].copy_from_slice(&1u16.to_le_bytes());bmp[28..30].copy_from_slice(&24u16.to_le_bytes());for row in 0..2{for x in 0..2{let i=54+row*8+x*3;bmp[i..i+3].copy_from_slice(&[30,100,220]);}}
        let art=decode(&bmp).unwrap();assert_eq!((art.width,art.height),(256,256));assert_eq!(art.pixels.len(),256*256*4);assert_eq!(art.pixels[3],0);let center=(128*256+128)*4;assert_eq!(art.pixels[center+3],255);assert!(art.pixels[center+2]>200);
    }
}
