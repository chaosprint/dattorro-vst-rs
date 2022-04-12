//! Barebones baseview egui plugin

#[macro_use]
extern crate vst;

use egui::CtxRef;

use baseview::{Size, WindowHandle, WindowOpenOptions, WindowScalePolicy};
use vst::buffer::AudioBuffer;
use vst::editor::Editor;
use vst::plugin::{Category, Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;

use egui_baseview::{EguiWindow, Queue, RenderSettings, Settings};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use std::sync::Arc;

use glicol_synth::{
    Message, AudioContext,
    oscillator::{SinOsc}, filter::{ OnePole, AllPassFilterGain}, effect::Balance,
    operator::{Mul, Add}, delay::{DelayN, DelayMs}, Pass,
    AudioContextBuilder, Sum
};

const WINDOW_WIDTH: usize = 300;
const WINDOW_HEIGHT: usize = 200;


// we have only struct definition in this lib
struct TestPluginEditor {
    params: Arc<GainEffectParameters>,
    window_handle: Option<WindowHandle>,
    is_open: bool,
}

struct GainEffectParameters {
    bandwidth: AtomicFloat,
    damping: AtomicFloat,
    decay: AtomicFloat,
    mix: AtomicFloat,
}
struct DattorroPlugin {
    params: Arc<GainEffectParameters>,
    editor: Option<TestPluginEditor>,
    context: AudioContext<128>,
    bandwidth: f32, // for checking the diff
    damping: f32,
    decay: f32,
    mix: f32,
}

impl Editor for TestPluginEditor {
    fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    fn size(&self) -> (i32, i32) {
        (WINDOW_WIDTH as i32, WINDOW_HEIGHT as i32)
    }

    fn open(&mut self, parent: *mut ::std::ffi::c_void) -> bool {
        ::log::info!("Editor open");
        if self.is_open {
            return false;
        }

        self.is_open = true;

        let settings = Settings {
            window: WindowOpenOptions {
                title: String::from("Dattorro Reverb"),
                size: Size::new(WINDOW_WIDTH as f64, WINDOW_HEIGHT as f64),
                scale: WindowScalePolicy::SystemScaleFactor,
            },
            render_settings: RenderSettings::default(),
        };

        let window_handle = EguiWindow::open_parented(
            &VstParent(parent),
            settings,
            self.params.clone(),
            |_egui_ctx: &CtxRef, _queue: &mut Queue, _state: &mut Arc<GainEffectParameters>| {},
            |egui_ctx: &CtxRef, _queue: &mut Queue, state: &mut Arc<GainEffectParameters>| {
                egui::Window::new("Dattorro Reverb").show(&egui_ctx, |ui| {
                    ui.heading("Made with egui and glicol_synth");
                    let mut bandwidth = state.bandwidth.get();
                    let mut damping = state.damping.get();
                    let mut decay = state.decay.get();
                    let mut mix = state.mix.get();
                    if ui
                        .add(egui::Slider::new(&mut bandwidth, 0.0..=1.0).text("bandwidth"))
                        .changed()
                    {
                        state.bandwidth.set(bandwidth)
                    }
                    if ui
                        .add(egui::Slider::new(&mut damping, 0.0..=1.0).text("damping"))
                        .changed()
                    {
                        state.damping.set(damping)
                    }
                    if ui
                        .add(egui::Slider::new(&mut decay, 0.0..=0.9999).text("decay"))
                        .changed()
                    {
                        state.decay.set(decay)
                    }
                    if ui
                        .add(egui::Slider::new(&mut mix, 0.0..=1.0).text("mix"))
                        .changed()
                    {
                        state.mix.set(mix)
                    }
                });
            },
        );

        self.window_handle = Some(window_handle);
        true
    }

    fn is_open(&mut self) -> bool {
        self.is_open
    }

    fn close(&mut self) {
        self.is_open = false;
        if let Some(mut window_handle) = self.window_handle.take() {
            window_handle.close();
        }
    }
}

impl Default for DattorroPlugin {
    fn default() -> Self {
        let params = Arc::new(GainEffectParameters::default());
        let mut context = AudioContextBuilder::<128>::new()
        .sr(48000).channels(2).build(); // todo: sr can be different
        

        // we create the input manually, and tag it for later use
        // you will see the tag usage soon
        let input = context.add_mono_node( Pass{} );
        context.tags.insert("input", input);

        let wet1 = context.add_mono_node(OnePole::new(0.7));
        context.tags.insert("inputlpf", wet1);
        let wet2 = context.add_mono_node(DelayMs::new().delay(50.));
        let wet3 = context.add_mono_node(AllPassFilterGain::new().delay(4.771).gain(0.75));
        let wet4 = context.add_mono_node(AllPassFilterGain::new().delay(3.595).gain(0.75));
        let wet5 = context.add_mono_node(AllPassFilterGain::new().delay(12.72).gain(0.625));
        let wet6 = context.add_mono_node(AllPassFilterGain::new().delay(9.307).gain(0.625));
        let wet7 = context.add_mono_node(Add::new(0.0)); // fb here
        let wet8 = context.add_mono_node(AllPassFilterGain::new().delay(100.0).gain(0.7)); // mod here

        context.chain(vec![input, wet1, wet2, wet3, wet4, wet5, wet6, wet7, wet8]);

        let mod1 = context.add_mono_node(SinOsc::new().freq(0.1));
        let mod2 = context.add_mono_node(Mul::new(5.5));
        let mod3 = context.add_mono_node(Add::new(29.5));
        let _ = context.chain(vec![mod1, mod2, mod3, wet8]);

        // we are going to take some halfway delay from line a
        let aa = context.add_mono_node(DelayN::new(394));
        context.connect(wet8, aa);
        let ab = context.add_mono_node(DelayN::new(2800));
        context.connect(aa, ab);
        let ac = context.add_mono_node(DelayN::new(1204));
        context.connect(ab, ac);


        let ba1 = context.add_mono_node( DelayN::new(2000));
        context.connect(ac, ba1);
        let ba2 = context.add_mono_node( OnePole::new(0.1));
        context.tags.insert("tanklpf1", ba2);
        context.connect(ba1, ba2);
        let ba3 = context.add_mono_node( AllPassFilterGain::new().delay(7.596).gain(0.5) );
        context.connect(ba2, ba3);

        let bb = context.add_mono_node(AllPassFilterGain::new().delay(35.78).gain(0.5));
        context.connect(ba3, bb);
        let bc = context.add_mono_node(AllPassFilterGain::new().delay(100.).gain(0.5));
        context.connect(bb, bc);
        let _ = context.chain(vec![mod1, mod2, mod3, bc]); // modulate here

        let ca = context.add_mono_node(DelayN::new(179));
        context.connect(bc, ca);
        let cb = context.add_mono_node(DelayN::new(2679));
        context.connect(ca, cb);
        let cc1 = context.add_mono_node(DelayN::new(3500));
        let cc2 = context.add_mono_node(Mul::new(0.3)); // another g5
        context.tags.insert("fbrate2", cc2);
        context.chain(vec![cb, cc1, cc2]);
        
        let da1 = context.add_mono_node(AllPassFilterGain::new().delay(30.).gain(0.7));
        let da2 = context.add_mono_node(DelayN::new(522));
        context.chain(vec![cc2, da1, da2]);
        
        let db = context.add_mono_node(DelayN::new(2400));
        context.connect(da2, db);
        let dc = context.add_mono_node(DelayN::new(2400));
        context.connect(db, dc);

        let ea1 = context.add_mono_node(OnePole::new(0.1));
        context.tags.insert("tanklpf2", ea1);
        let ea2 = context.add_mono_node(AllPassFilterGain::new().delay(6.2).gain(0.7));
        context.chain(vec![dc, ea1, ea2]);

        let eb = context.add_mono_node(AllPassFilterGain::new().delay(34.92).gain(0.7));
        context.connect(ea2, eb);

        let fa1 = context.add_mono_node(AllPassFilterGain::new().delay(20.4).gain(0.7));
        let fa2 = context.add_mono_node(DelayN::new(1578));
        context.chain(vec![eb, fa1, fa2]);
        let fb = context.add_mono_node(DelayN::new(2378));
        context.connect(fa2, fb);

        let fb1 = context.add_mono_node(DelayN::new(2500));
        let fb2 = context.add_mono_node(Mul::new(0.3));
        context.tags.insert("fbrate1", fb2);
        context.chain(vec![fb, fb1, fb2, wet7]); // back to feedback
        

        // start to take some signal out
        let left_subtract = context.add_mono_node( Sum{});
        context.connect(bb,left_subtract);
        context.connect(db,left_subtract);
        context.connect(ea2,left_subtract);
        context.connect(fa2,left_subtract);

        // turn these signal into -
        let left_subtract2 = context.add_mono_node(Mul::new(-1.0));
        context.connect(left_subtract,left_subtract2);
        
        let left = context.add_mono_node(Sum{});
        context.connect(aa,left);
        context.connect(ab,left);
        context.connect(cb,left);
        context.connect(left_subtract2,left);
        let leftwet = context.add_mono_node(Mul::new(0.1));
        context.tags.insert("mix1", leftwet);
        let leftmix = context.add_mono_node(Sum{});
        
        // input dry * (1.-mix)
        let leftdrymix = context.add_mono_node(Mul::new(0.9));
        context.tags.insert("mixdiff1", leftdrymix);
        context.chain(vec![input, leftdrymix, leftmix]);
        context.chain(vec![left, leftwet, leftmix]);
        
        let right_subtract = context.add_mono_node(Sum{});
        context.connect(eb,right_subtract);
        context.connect(ab,right_subtract);
        context.connect(ba2,right_subtract);
        context.connect(ca,right_subtract);
        let right_subtract2 = context.add_mono_node(Mul::new(-1.0));
        context.connect(right_subtract,right_subtract2);

        let right = context.add_mono_node(Sum{});
        context.connect(da2,right);
        context.connect(db,right);
        context.connect(fb,right);
        context.connect(right_subtract2,right);
        let rightwet = context.add_mono_node(Mul::new(0.1));
        context.tags.insert("mix2", rightwet);
        let rightmix = context.add_mono_node(Sum{}); // input dry * (1.-mix)

        let rightdry = context.add_mono_node(Mul::new(0.9));
        context.tags.insert("mixdiff2", rightdry);
        context.chain(vec![input, rightdry, rightmix]);
        context.chain(vec![right, rightwet,rightmix]);
        
        let balance = context.add_stereo_node(Balance::new());
        context.connect(leftmix,balance);
        context.connect(rightmix,balance);
        context.connect(balance, context.destination);

        Self {
            params: params.clone(),
            editor: Some(TestPluginEditor {
                params: params.clone(),
                window_handle: None,
                is_open: false,
            }),
            context,
            bandwidth: 0.7,
            damping: 0.1,
            decay: 0.3,
            mix: 0.1
        }
    }
}

impl Default for GainEffectParameters {
    fn default() -> GainEffectParameters {
        GainEffectParameters {
            bandwidth: AtomicFloat::new(0.7),
            damping: AtomicFloat::new(0.1),
            decay: AtomicFloat::new(0.3),
            mix: AtomicFloat::new(0.1),
        }
    }
}

impl Plugin for DattorroPlugin {
    fn get_info(&self) -> Info {
        Info {
            name: "Dattorro Reverb".to_string(),
            vendor: "chaosprint".to_string(),
            unique_id: 19891010,
            version: 1,
            inputs: 1, // channels, dattorro is mono in, stereo out
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 4,
            category: Category::Effect,
            ..Default::default()
        }
    }

    fn init(&mut self) {
        let log_folder = ::dirs::home_dir().unwrap().join("tmp");

        let _ = ::std::fs::create_dir(log_folder.clone());

        let log_file = ::std::fs::File::create(log_folder.join("EGUIBaseviewTest.log")).unwrap();

        let log_config = ::simplelog::ConfigBuilder::new()
            .set_time_to_local(true)
            .build();

        let _ = ::simplelog::WriteLogger::init(simplelog::LevelFilter::Info, log_config, log_file);

        ::log_panics::init();

        ::log::info!("init");
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        if let Some(editor) = self.editor.take() {
            Some(Box::new(editor) as Box<dyn Editor>)
        } else {
            None
        }
    }

    // Here is where the bulk of our audio processing code goes.
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {

        let bandwidth = self.params.bandwidth.get();
        let damping = self.params.damping.get();
        let decay = self.params.decay.get();
        let mix = self.params.mix.get();

        if bandwidth != self.bandwidth {
            self.context.send_msg(self.context.tags["inputlpf"], Message::SetToNumber(0, bandwidth));
            self.bandwidth = bandwidth;
        }

        if damping != self.damping {
            self.context.send_msg(self.context.tags["tanklpf1"], Message::SetToNumber(0, damping));
            self.context.send_msg(self.context.tags["tanklpf2"], Message::SetToNumber(0, damping));
            self.damping = damping;
        }

        if decay != self.decay {
            self.context.send_msg(self.context.tags["fbrate1"], Message::SetToNumber(0, decay));
            self.context.send_msg(self.context.tags["fbrate2"], Message::SetToNumber(0, decay));
            self.decay = decay;
        }

        if mix != self.mix {
            self.context.send_msg(self.context.tags["mix1"], Message::SetToNumber(0, mix));
            self.context.send_msg(self.context.tags["mix2"], Message::SetToNumber(0, mix));
            self.context.send_msg(self.context.tags["mixdiff1"], Message::SetToNumber(0, 1.-mix));
            self.context.send_msg(self.context.tags["mixdiff2"], Message::SetToNumber(0, 1.-mix));
            self.mix = mix;
        }

        let block_size: usize = buffer.samples();

        let (input, mut outputs) = buffer.split();
        let output_channels = outputs.len();
        let process_times = block_size / 128;

        for b in 0..process_times {
            let inp =  &input.get(0)[b*128..(b+1)*128];

            self.context.graph[
                self.context.tags["input"]
            ].buffers[0].copy_from_slice(inp);

            // self.context.graph[
            //     self.context.tags["input"]
            // ].buffers[1].copy_from_slice(inp[1]);

            let engine_out = self.context.next_block();

            for chan_idx in 0..output_channels {
                let buff = outputs.get_mut(chan_idx);
                for n in 0..128 {
                    buff[b*128+n] = engine_out[chan_idx][n];
                }
            }
        }
    }

    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

impl PluginParameters for GainEffectParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.bandwidth.get(),
            1 => self.damping.get(),
            2 => self.decay.get(),
            3 => self.mix.get(),
            _ => 0.0,
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        #[allow(clippy::single_match)]
        match index {
            0 => self.bandwidth.set(val),
            1 => self.damping.set(val),
            2 => self.decay.set(val),
            3 => self.mix.set(val),
            _ => (),
        }
    }

    // we avoid default controller for now

    // // This is what will display underneath our control.  We can
    // // format it into a string that makes the most since.
    // fn get_parameter_text(&self, index: i32) -> String {
    //     match index {
    //         0 => format!("{:.2}", (self.mix.get() - 0.5) * 2f32),
    //         _ => "".to_string(),
    //     }
    // }

    // This shows the control's name.
    // fn get_parameter_name(&self, index: i32) -> String {
    //     match index {
    //         0 => "Mix",
    //         _ => "",
    //     }
    //     .to_string()
    // }
}


// boilerplate code, identical for all vst plugins
// just skip to the last line
struct VstParent(*mut ::std::ffi::c_void);

#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}

#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}

#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}

// the last thing you need to change is the name
plugin_main!(DattorroPlugin);