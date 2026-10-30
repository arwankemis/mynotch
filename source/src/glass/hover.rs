use crate::motion::SpringScalar;
/// Percent coordinates keep the geometry spring's settle tolerance meaningful at any DPI.
pub struct PointerSheen { x:SpringScalar,y:SpringScalar,strength:SpringScalar }
impl Default for PointerSheen {
    fn default()->Self{Self{x:SpringScalar::new(50.,170.,26.1),y:SpringScalar::new(0.,170.,26.1),strength:SpringScalar::new(0.,170.,26.1)}}
}
impl PointerSheen {
    pub fn target(&mut self,point:Option<(f32,f32)>,reduced:bool){
        let (x,y,a)=point.map(|(x,y)|(x.clamp(0.,1.)*100.,y.clamp(0.,1.)*100.,100.)).unwrap_or((50.,0.,0.));
        if reduced{self.x.snap(x);self.y.snap(y);self.strength.snap(a);}else{self.x.set_target(x);self.y.set_target(y);self.strength.set_target(a);}
    }
    pub fn step(&mut self,dt:f32)->bool{self.x.step(dt)|self.y.step(dt)|self.strength.step(dt)}
    pub fn sample(&self)->(f32,f32,f32){(self.x.current/100.,self.y.current/100.,self.strength.current.clamp(0.,100.)/100.)}
}
#[cfg(test)] mod tests{
    use super::*;
    #[test]fn pointer_sheen_retargets_without_jumping_and_stops_at_idle(){
        let mut s=PointerSheen::default();s.target(Some((0.8,0.6)),false);s.step(0.016);let before=s.sample();
        s.target(Some((0.2,0.8)),false);assert_eq!(before,s.sample());
        let mut moving=true;for _ in 0..180{moving=s.step(1./60.);}
        assert!(!moving);assert_eq!(s.sample(),(0.2,0.8,1.));
        s.target(None,false);for _ in 0..180{s.step(1./60.);}assert_eq!(s.sample().2,0.);
    }
    #[test]fn reduced_motion_snaps_without_an_animation_timer(){let mut s=PointerSheen::default();s.target(Some((1.,1.)),true);assert_eq!(s.sample(),(1.,1.,1.));assert!(!s.step(0.));}
}
