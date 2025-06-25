//! Web-based Anime4K video player
//!
//! A browser-compatible video player that processes video files using WebGPU
//! for real-time Anime4K upscaling in the browser.

use anime4k_wgpu::presets::{Anime4KPerformancePreset, Anime4KPreset};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Element, HtmlCanvasElement, HtmlElement, HtmlInputElement};


mod player;
mod utils;

use player::VideoPlayer;
use utils::set_panic_hook;

/// Entry point for the web application
#[wasm_bindgen(start)]
pub fn run() {
    set_panic_hook();

    web_sys::console::log_1(&"Panic hook set".into());

    // Skip tracing for now to avoid potential issues
    // tracing_wasm::set_as_global_default();

    web_sys::console::log_1(&"Starting Anime4K-wgpu...".into());

    spawn_local(async {
        if let Err(e) = run_app().await {
            web_sys::console::error_1(&format!("App error: {:?}", e).into());
        }
    });
}

/// Main application initialization
async fn run_app() -> Result<(), JsValue> {
    web_sys::console::log_1(&"Getting window...".into());
    let web_window = web_sys::window().ok_or("No window found")?;

    web_sys::console::log_1(&"Getting document...".into());
    let document = web_window.document().ok_or("No document found")?;

    // Create canvas element
    web_sys::console::log_1(&"Creating canvas...".into());
    let canvas = document.create_element("canvas")?.dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(1280);
    canvas.set_height(720);
    canvas.style().set_property("display", "block")?;
    canvas.style().set_property("margin", "auto")?;

    // Insert canvas into DOM - only if it doesn't exist
    let container = document.get_element_by_id("canvas-container").unwrap_or_else(|| {
        let container = document.create_element("div").unwrap();
        container.set_id("canvas-container");
        document.body().unwrap().append_child(&container).unwrap();
        container
    });

    // Clear existing content and add canvas
    container.set_inner_html("");
    container.append_child(&canvas)?;

    // Initialize video player
    web_sys::console::log_1(&"Creating video player...".into());
    let player = Rc::new(RefCell::new(VideoPlayer::new(canvas).await));

    // Clear controls once before setup to avoid duplicates
    if let Some(controls) = document.get_element_by_id("controls") {
        controls.set_inner_html("");
    }

    // Set up video input for file loading
    setup_video_input(&document, player.clone()).await?;

    // Set up preset controls
    setup_preset_controls(&document, player.clone())?;

    // Start render loop
    start_render_loop(player);

    Ok(())
}

/// Sets up video file input
async fn setup_video_input(document: &Document, player: Rc<RefCell<VideoPlayer>>) -> Result<(), JsValue> {
    let input = document.create_element("input")?.dyn_into::<HtmlInputElement>()?;
    input.set_type("file");
    input.set_accept("video/*");
    input.set_id("video-input");

    let label = document.create_element("label")?;
    label.set_text_content(Some("Choose a video file: "));
    label.set_attribute("for", "video-input")?;

    let controls = get_or_create_controls_container(document)?;
    controls.append_child(&label)?;
    controls.append_child(&input)?;
    controls.append_child(&document.create_element("br")?.into())?;

    // Handle file selection
    let player_clone = player.clone();
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let input = event.target().unwrap().dyn_into::<HtmlInputElement>().unwrap();
        let files = input.files().unwrap();

        if files.length() > 0 {
            let file = files.get(0).unwrap();
            let player_ref = player_clone.clone();

            spawn_local(async move {
                if let Ok(video_url) = create_video_url(file).await {
                    player_ref.borrow_mut().load_video(&video_url);
                }
            });
        }
    }) as Box<dyn FnMut(_)>);

    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
    closure.forget();

    Ok(())
}

/// Sets up preset control buttons
fn setup_preset_controls(document: &Document, player: Rc<RefCell<VideoPlayer>>) -> Result<(), JsValue> {
    let controls = get_or_create_controls_container(document)?;

    // Video controls
    let video_controls_div = document.create_element("div")?;
    video_controls_div.set_class_name("video-controls");

    // Test video loading button
    let test_video_button = document.create_element("button").unwrap();
    test_video_button.set_text_content(Some("Load Test Video"));
    test_video_button.set_id("test-video-btn");

    let player_test = player.clone();
    let test_video_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        // Use a publicly available test video (Big Buck Bunny sample)
        let test_video_url = "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4";
        player_test.borrow_mut().load_video(test_video_url);
        web_sys::console::log_1(&"Loading test video...".into());
    }) as Box<dyn FnMut(_)>);

    test_video_button.dyn_ref::<HtmlElement>().unwrap().set_onclick(Some(test_video_closure.as_ref().unchecked_ref()));
    test_video_closure.forget();

    let play_button = document.create_element("button").unwrap();
    play_button.set_text_content(Some("Play"));
    play_button.set_id("play-btn");

    let pause_button = document.create_element("button").unwrap();
    pause_button.set_text_content(Some("Pause"));
    pause_button.set_id("pause-btn");

    let player_clone = player.clone();
    let play_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        player_clone.borrow_mut().play();
    }) as Box<dyn FnMut(_)>);

    let player_clone2 = player.clone();
    let pause_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        player_clone2.borrow_mut().pause();
    }) as Box<dyn FnMut(_)>);

    play_button.dyn_ref::<HtmlElement>().unwrap().set_onclick(Some(play_closure.as_ref().unchecked_ref()));
    pause_button.dyn_ref::<HtmlElement>().unwrap().set_onclick(Some(pause_closure.as_ref().unchecked_ref()));
    play_closure.forget();
    pause_closure.forget();

    video_controls_div.append_child(&test_video_button).unwrap();
    video_controls_div.append_child(&play_button).unwrap();
    video_controls_div.append_child(&pause_button).unwrap();
    controls.append_child(&video_controls_div).unwrap();
    controls.append_child(&document.create_element("br").unwrap()).unwrap();

    // Anime4K preset controls
    let preset_label = document.create_element("label").unwrap();
    preset_label.set_text_content(Some("Anime4K Preset: "));
    controls.append_child(&preset_label).unwrap();

    let presets = [
        ("Off", Anime4KPreset::Off),
        ("Mode A", Anime4KPreset::ModeA),
        ("Mode AA", Anime4KPreset::ModeAA),
        ("Mode B", Anime4KPreset::ModeB),
        ("Mode BB", Anime4KPreset::ModeBB),
        ("Mode C", Anime4KPreset::ModeC),
        ("Mode CA", Anime4KPreset::ModeCA),
    ];

    for (name, preset) in presets {
        let button = document.create_element("button").unwrap();
        button.set_text_content(Some(name));

        let player_clone = player.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            player_clone.borrow_mut().set_anime4k_preset(preset);
            web_sys::console::log_1(&format!("Setting preset to {}", name).into());
        }) as Box<dyn FnMut(_)>);

        button.dyn_ref::<HtmlElement>().unwrap().set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        controls.append_child(&button).unwrap();
    }

    controls.append_child(&document.create_element("br").unwrap()).unwrap();

    // Performance preset controls
    let perf_label = document.create_element("label").unwrap();
    perf_label.set_text_content(Some("Performance: "));
    controls.append_child(&perf_label).unwrap();

    let perf_presets = [
        ("Light", Anime4KPerformancePreset::Light),
        ("Medium", Anime4KPerformancePreset::Medium),
        ("High", Anime4KPerformancePreset::High),
        ("Ultra", Anime4KPerformancePreset::Ultra),
        ("Extreme", Anime4KPerformancePreset::Extreme),
    ];

    for (name, preset) in perf_presets {
        let button = document.create_element("button").unwrap();
        button.set_text_content(Some(name));

        let player_clone = player.clone();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            player_clone.borrow_mut().set_anime4k_performance_preset(preset);
            web_sys::console::log_1(&format!("Setting performance preset to {}", name).into());
        }) as Box<dyn FnMut(_)>);

        button.dyn_ref::<HtmlElement>().unwrap().set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();

        controls.append_child(&button).unwrap();
    }

    Ok(())
}

/// Gets or creates the controls container element
fn get_or_create_controls_container(document: &Document) -> Result<Element, JsValue> {
    let controls = document.get_element_by_id("controls").unwrap_or_else(|| {
        let controls = document.create_element("div").unwrap();
        controls.set_id("controls");
        controls.dyn_ref::<HtmlElement>().unwrap().style().set_property("text-align", "center").unwrap();
        controls.dyn_ref::<HtmlElement>().unwrap().style().set_property("margin", "20px").unwrap();
        document.body().unwrap().append_child(&controls).unwrap();
        controls
    });

    Ok(controls)
}

/// Creates a video URL from a File object
async fn create_video_url(file: web_sys::File) -> Result<String, JsValue> {
    let url = web_sys::Url::create_object_url_with_blob(&file)?;
    Ok(url)
}

/// Starts the render loop using requestAnimationFrame
fn start_render_loop(player: Rc<RefCell<VideoPlayer>>) {
    fn request_animation_frame(f: &Closure<dyn FnMut()>) {
        web_sys::window()
            .unwrap()
            .request_animation_frame(f.as_ref().unchecked_ref())
            .expect("should register `requestAnimationFrame` OK");
    }

    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        player.borrow_mut().render();
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}
