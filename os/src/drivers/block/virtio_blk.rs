//! Block driver for the chapter 9 API exercise.
//!
//! Keep the constructor and token-to-condition-variable table provided.
//! Use polling before tasks start, and completion interrupts afterwards.
//! Request submission and wait registration share one protected access;
//! release the device borrow before scheduling, keeping buffers alive.

#![allow(dead_code, unused_imports, unused_variables)]

use super::BlockDevice;
use crate::DEV_NON_BLOCKING_ACCESS;
use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::{Condvar, UPIntrFreeCell};
use crate::task::schedule;
use alloc::collections::BTreeMap;
use virtio_drivers::{BlkResp, RespStatus, VirtIOBlk, VirtIOHeader};

#[allow(unused)]
const VIRTIO0: usize = 0x10008000;

pub struct VirtIOBlock {
    virtio_blk: UPIntrFreeCell<VirtIOBlk<'static, VirtioHal>>,
    condvars: BTreeMap<u16, Condvar>,
}

impl BlockDevice for VirtIOBlock {
    /// Read one block into the caller's buffer, returning only on completion.
    ///
    /// DEV_NON_BLOCKING_ACCESS selects the existing polling or interrupt path.
    /// In the latter, wait on the submitted token and check the response after
    /// resumption. Keep the response and buffer valid while the device uses them.
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        todo!("drivers::block::VirtIOBlock::read_block")
    }
    /// Write one block from the caller's buffer, returning only on completion.
    ///
    /// Follow the same mode selection, protected wait registration, borrow
    /// release, and response validation requirements as read_block().
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        todo!("drivers::block::VirtIOBlock::write_block")
    }
    /// Consume every currently completed token and signal its condition variable.
    /// Do not wait for future completions or switch tasks in this handler.
    fn handle_irq(&self) {
        todo!("drivers::block::VirtIOBlock::handle_irq")
    }
}

impl VirtIOBlock {
    pub fn new() -> Self {
        let virtio_blk = unsafe {
            UPIntrFreeCell::new(
                VirtIOBlk::<VirtioHal>::new(&mut *(VIRTIO0 as *mut VirtIOHeader)).unwrap(),
            )
        };
        let mut condvars = BTreeMap::new();
        let channels = virtio_blk.exclusive_access().virt_queue_size();
        for i in 0..channels {
            let condvar = Condvar::new();
            condvars.insert(i, condvar);
        }
        Self {
            virtio_blk,
            condvars,
        }
    }
}
