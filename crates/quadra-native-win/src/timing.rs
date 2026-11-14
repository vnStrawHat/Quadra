//! What a frame costs the GPU (specs/07, "Drawing a frame"): two timestamps around a frame's
//! draw, read back a few frames later without waiting for the GPU.

use std::ffi::c_void;
use std::time::Duration;

use windows::Win32::Foundation::S_OK;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_ASYNC_GETDATA_DONOTFLUSH, D3D11_QUERY, D3D11_QUERY_DATA_TIMESTAMP_DISJOINT, D3D11_QUERY_DESC,
    D3D11_QUERY_TIMESTAMP, D3D11_QUERY_TIMESTAMP_DISJOINT, ID3D11Device, ID3D11DeviceContext, ID3D11Query,
};
use windows::core::Interface;

/// Frames measured at once: a frame is not measured while the GPU is this many behind.
const RING: usize = 4;

struct Set {
    disjoint: ID3D11Query,
    start: ID3D11Query,
    end: ID3D11Query,
    /// The frame it measures, while its answer has not come back.
    waiting: Option<u64>,
}

#[derive(Default)]
pub(crate) struct Timing {
    sets: Vec<Set>,
    measuring: Option<usize>,
    frame: u64,
    /// The latest frame whose time came back, and that time.
    pub last: Option<(u64, Duration)>,
}

fn query(device: &ID3D11Device, kind: D3D11_QUERY) -> Option<ID3D11Query> {
    let mut out = None;
    // SAFETY: the description and the out slot are alive for the call.
    unsafe { device.CreateQuery(&D3D11_QUERY_DESC { Query: kind, MiscFlags: 0 }, Some(&mut out)) }.ok()?;
    out
}

/// A query's answer, if the GPU has given it: never waits.
fn answer<T: Default>(context: &ID3D11DeviceContext, query: &ID3D11Query) -> Option<T> {
    let mut data = T::default();
    // The wrapper takes "not yet" for success: the code is read here.
    // SAFETY: `data` is as large as the size given, and both objects are alive.
    let code = unsafe {
        (Interface::vtable(context).GetData)(
            Interface::as_raw(context),
            query.as_raw(),
            &mut data as *mut T as *mut c_void,
            size_of::<T>() as u32,
            D3D11_ASYNC_GETDATA_DONOTFLUSH.0 as u32,
        )
    };
    (code == S_OK).then_some(data)
}

impl Timing {
    /// Before a frame's draw: takes in what came back, and starts measuring if a set is free.
    pub fn begin(&mut self, device: &ID3D11Device, context: &ID3D11DeviceContext) {
        self.frame += 1;
        self.take_answers(context);
        if self.sets.len() < RING
            && let (Some(disjoint), Some(start), Some(end)) = (
                query(device, D3D11_QUERY_TIMESTAMP_DISJOINT),
                query(device, D3D11_QUERY_TIMESTAMP),
                query(device, D3D11_QUERY_TIMESTAMP),
            )
        {
            self.sets.push(Set { disjoint, start, end, waiting: None });
        }
        self.measuring = self.sets.iter().position(|set| set.waiting.is_none());
        if let Some(set) = self.measuring.map(|i| &self.sets[i]) {
            // SAFETY: plain calls on live queries.
            unsafe {
                context.Begin(&set.disjoint);
                context.End(&set.start);
            }
        }
    }

    /// After a frame's draw.
    pub fn end(&mut self, context: &ID3D11DeviceContext) {
        let Some(set) = self.measuring.take().map(|i| &mut self.sets[i]) else { return };
        // SAFETY: plain calls on live queries.
        unsafe {
            context.End(&set.end);
            context.End(&set.disjoint);
        }
        set.waiting = Some(self.frame);
    }

    fn take_answers(&mut self, context: &ID3D11DeviceContext) {
        for set in &mut self.sets {
            let Some(frame) = set.waiting else { continue };
            let Some(clock) = answer::<D3D11_QUERY_DATA_TIMESTAMP_DISJOINT>(context, &set.disjoint) else {
                continue;
            };
            set.waiting = None;
            let (start, end) = (answer::<u64>(context, &set.start), answer::<u64>(context, &set.end));
            let (Some(start), Some(end), false) = (start, end, clock.Disjoint.as_bool()) else { continue };
            if clock.Frequency > 0 && self.last.is_none_or(|(at, _)| at < frame) {
                let seconds = end.saturating_sub(start) as f64 / clock.Frequency as f64;
                self.last = Some((frame, Duration::from_secs_f64(seconds)));
            }
        }
    }
}
