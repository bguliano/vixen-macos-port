// Appended only to an immutable private copy of production picture.rs.
// Calls original decode_frames and Picture.setup_render; observes cache before
// render_effect consumes it. The empty adapter render is not effect-output proof.
pub fn capture_private_gif(root: &std::path::Path, file: &str, speed: i32) -> serde_json::Value {
    use serde_json::json;
    fn pixels(b: &Bitmap) -> Vec<u8> {
        let mut bytes=Vec::new();
        for y in 0..b.height() { for x in 0..b.width() {
            let c=b.get_pixel(x,y); bytes.extend_from_slice(&[c.a,c.r,c.g,c.b]);
        }} bytes
    }
    struct Capture { original: Picture, rows: Vec<serde_json::Value> }
    impl PixelEffect for Capture {
        fn setup_render(&mut self, info: &PixelInfo) {
            assert_eq!((info.buffer_wi(),info.buffer_ht(),info.number_frames()),(4,4,12));
            self.original.setup_render(info);
            self.rows=self.original.pictures.iter().map(|p| json!({"effectFrame":p.frame,"width":p.bitmap.width(),"height":p.bitmap.height(),"argb":pixels(&p.bitmap)})).collect();
        }
        fn render_effect(&mut self, _frame:i32, _info:&PixelInfo, _buf:&mut PixelFrameBuffer) {}
        fn cleanup_render(&mut self) { self.original.cleanup_render(); }
    }
    let input=std::fs::read(root.join("Picture").join(file)).unwrap();
    let decoded=decode_frames(&input);
    assert_eq!(decoded.len(),4);
    let decoded:Vec<_>=decoded.iter().enumerate().map(|(i,b)|json!({"sourceFrame":i,"width":b.width(),"height":b.height(),"argb":pixels(b)})).collect();
    let mut tree=ElementTree::new();
    let target=vixen_effects::testing::grid(&mut tree,"Private GIF",4,4);
    let cancel=std::sync::atomic::AtomicBool::new(false);
    let ctx=RenderCtx {tree:&tree, targets:&[target],start_time:TimeSpan::ZERO,time_span:TimeSpan::from_ms_i(600),frame_ms:50,marks:&[],audio:None,module_data_root:Some(root),video_cache:None,cancel:&cancel};
    let mut p=DESCRIPTOR.new_params();
    p.set("Source",ParamValue::Enum("File".into())); p.set("FileName",ParamValue::Str(file.into()));
    p.set("GifSpeed",ParamValue::Int(speed)); p.set("ScalePercent",ParamValue::Int(100));
    p.set("ScaleToGrid",ParamValue::Bool(false)); p.set("StretchToGrid",ParamValue::Bool(false));
    p.set("ColorEffect",ParamValue::Enum("None".into()));
    let mut capture=Capture{original:from_params(&ctx,&p),rows:Vec::new()};
    let _=render_pixel(&ctx,StringOrientation::Horizontal,TargetPositioning::Strings,&mut capture);
    json!({"file":file,"gifSpeed":speed,"decodedFrames":decoded,"cache":capture.rows})
}
