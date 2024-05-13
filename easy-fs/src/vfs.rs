use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};

/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        if !disk_inode.is_dir() {
            return None; // Not a directory
        }

        let file_count = disk_inode.size as usize / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            if let Ok(size) = disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device) {
                if size != DIRENT_SZ {
                    continue; // Unable to read dirent
                }
                if dirent.name() == name {
                    return Some(dirent.inode_id() as u32);
                }
            }
        }
        None
    }

    fn find_dirent_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        if !disk_inode.is_dir() {
            return None; // Not a directory
        }

        let file_count = disk_inode.size as usize / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            if let Ok(size) = disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device) {
                if size != DIRENT_SZ {
                    continue; // Unable to read dirent
                }
                if dirent.name() == name {
                    return Some(i as u32);
                }
            }
        }
        None
    }

    pub fn find_inode(&self, name: &str) -> Option<u32> {
        let mut inode_id = 0;
        self.read_disk_inode(|disk_inode| {
            if let Some(id) = self.find_inode_id(name, disk_inode) {
                inode_id = id;
            }
        });
        Some(inode_id)
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            if let Some(inode_id) = self.find_inode_id(name, disk_inode) {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                return Some(Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                )));
            }
            None
        })
    }

    // Rest of the methods remain unchanged
    // ...
}
