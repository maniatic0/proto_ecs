use std::{
    mem,
    sync::atomic::{AtomicBool, Ordering},
};

use lazy_static::lazy_static;
use macaw::Mat4;
use parking_lot::RwLock;

use crate::{
    core::assets_management::models::ModelHandle, entities::transform_datagroup::TransformMatrix,
};

use super::{camera::Camera, material::MaterialHandle};

pub struct RenderThread {
    current_frame_desc: FrameDesc,
    // ? We only use RwLock on this member bc this is the one that is touched by both the render thread
    // and the main thread. Should the other frame be protected within a RwLock as well?
    last_frame_finished: AtomicBool,
    running: AtomicBool,
}

/// A description of a frame to render.
///
/// Holds the data required to render a scene, like all the
/// render proxies, the camera, light descriptions and so on
#[derive(Debug, Default)]
pub struct FrameDesc {
    pub render_proxies: Vec<RenderProxy>,
    pub camera: Camera, // Lights not yet implemented
}

/// Render proxies should be POD only, so that they can be easily be copied
/// and sent between threads
#[derive(Debug, Clone, Copy)]
pub struct RenderProxy {
    pub model: ModelHandle,
    pub material: MaterialHandle,
    pub transform: TransformMatrix,
}

lazy_static! {
    static ref RENDER_THREAD_STATE: RwLock<Option<RenderThread>> = RwLock::new(None);
}

lazy_static! {
    /// Description of the next frame.
    ///
    /// We use a variable independent of [RENDER_THREAD_STATE] to prevent the lock
    /// in that variable from locking the next frame
    static ref NEXT_FRAME_DESC : RwLock<FrameDesc> = RwLock::new(FrameDesc::default());
}

macro_rules! read_render_thread {
    ($i:ident) => {
        let __context__ = proto_ecs::core::rendering::render_thread::RENDER_THREAD_STATE.read();
        let $i = __context__
            .as_ref()
            .expect("Render thread not yet initialized!");
    };
}

macro_rules! write_render_thread {
    ($i:ident) => {
        let mut __context__ =
            proto_ecs::core::rendering::render_thread::RENDER_THREAD_STATE.write();
        let $i = __context__
            .as_mut()
            .expect("Render thread not yet initialized!");
    };
}

impl RenderThread {
    pub fn init() {
        let mut state = RENDER_THREAD_STATE.write();
        *state = Some(RenderThread::new());
    }

    fn new() -> Self {
        RenderThread {
            current_frame_desc: FrameDesc::default(),
            last_frame_finished: AtomicBool::new(true),
            running: AtomicBool::new(false),
        }
    }

    #[inline(always)]
    pub fn get_next_frame_desc() -> &'static RwLock<FrameDesc> {
        &NEXT_FRAME_DESC
    }

    pub fn run() {

        while RenderThread::is_running() {
            // TODO Change for a cond variable or something better
            // Busywaiting until last frame is outdated
            if RenderThread::is_last_frame_finished() {
                continue;
            }

            // Update the data used to draw the current frame
            RenderThread::update_current_frame_desc();
            

            // Actual rendering
            RenderThread::render();
        }
    }

    /// Mark the next frame data as updated.
    ///
    /// The render thread will not draw anything if the render thread is not
    /// updated
    pub fn next_frame_updated() {
        read_render_thread!(render_thread);
        render_thread
            .last_frame_finished
            .store(false, Ordering::SeqCst);
    }

    /// Stop the render thread
    pub fn stop() {
        read_render_thread!(render_thread);
        render_thread.running.store(false, Ordering::SeqCst);
    }

    fn update_current_frame_desc() {
        write_render_thread!(render_thread);
        let mut next_frame = NEXT_FRAME_DESC.write();
        mem::swap(&mut render_thread.current_frame_desc, &mut *next_frame);
    }

    #[inline(always)]
    fn is_running() -> bool {
        read_render_thread!(render_thread);
        render_thread.running.load(Ordering::SeqCst)
    }

    #[inline(always)]
    fn is_last_frame_finished() -> bool {
        read_render_thread!(render_thread);
        render_thread.last_frame_finished.load(Ordering::SeqCst)
    }

    fn render() {}
}
