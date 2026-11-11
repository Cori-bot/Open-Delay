use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Storage I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Media packet is larger than the history budget")]
    PacketTooLarge,
    #[error("Media packet has expired from the ring")]
    Expired,
    #[error("History capacity must be nonzero")]
    InvalidCapacity,
    #[error("Media sequence counter exhausted")]
    SequenceExhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketMeta {
    pub sequence: u64,
    pub route_id: String,
    pub generation: u64,
    pub timestamp_ms: u64,
    pub type_id: u8,
    pub csid: u32,
    pub stream_id: u32,
    pub keyframe: bool,
    pub configuration: bool,
    pub size: usize,
    #[serde(skip)]
    offset: u64,
}
#[derive(Debug, Clone)]
pub struct StoredPacket {
    pub meta: PacketMeta,
    pub payload: Arc<[u8]>,
}

/// One global ring shared by all routes. Metadata count and cached bytes are separately bounded.
/// The file is ephemeral: packet offsets are never persisted as recoverable session state.
pub struct DiskHistory {
    file: Option<File>,
    path: PathBuf,
    capacity: u64,
    cursor: u64,
    sequence: u64,
    used_bytes: u64,
    entries: VecDeque<PacketMeta>,
    max_entries: usize,
    regions: BTreeMap<u64, (u64, usize)>,
    cache: HashMap<u64, Arc<[u8]>>,
    cache_order: VecDeque<u64>,
    cache_capacity: usize,
    cache_bytes: usize,
}
impl DiskHistory {
    pub fn new(
        path: impl AsRef<Path>,
        capacity: u64,
        cache_capacity: usize,
    ) -> Result<Self, HistoryError> {
        if capacity == 0 {
            return Err(HistoryError::InvalidCapacity);
        }
        if let Some(parent) = path.as_ref().parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.set_len(capacity)?;
        Ok(Self {
            file: Some(file),
            path: path.as_ref().to_owned(),
            capacity,
            cursor: 0,
            sequence: 0,
            used_bytes: 0,
            entries: VecDeque::new(),
            max_entries: 200_000,
            regions: BTreeMap::new(),
            cache: HashMap::new(),
            cache_order: VecDeque::new(),
            cache_capacity,
            cache_bytes: 0,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn append(
        &mut self,
        route_id: &str,
        generation: u64,
        timestamp_ms: u64,
        type_id: u8,
        csid: u32,
        stream_id: u32,
        payload: &[u8],
        keyframe: bool,
        configuration: bool,
    ) -> Result<PacketMeta, HistoryError> {
        if payload.is_empty() || payload.len() as u64 > self.capacity {
            return Err(HistoryError::PacketTooLarge);
        }
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(HistoryError::SequenceExhausted)?;
        if self.cursor + payload.len() as u64 > self.capacity {
            self.cursor = 0;
        }
        let end = self.cursor + payload.len() as u64;
        // Removal can leave holes, so find every overwritten region rather than assuming
        // the first chronological entry overlaps. Drop through the newest overlap.
        let mut overwritten = self
            .regions
            .range(self.cursor..end)
            .map(|(_, entry)| entry.0)
            .max();
        if let Some((offset, (sequence, size))) = self.regions.range(..self.cursor).next_back() {
            if offset + *size as u64 > self.cursor {
                overwritten = Some(overwritten.map_or(*sequence, |last| last.max(*sequence)));
            }
        }
        while self
            .entries
            .front()
            .is_some_and(|p| overwritten.is_some_and(|last| p.sequence <= last))
            || self.entries.len() >= self.max_entries
        {
            self.evict_front();
        }
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| std::io::Error::other("History is closed"))?;
        file.seek(SeekFrom::Start(self.cursor))?;
        file.write_all(payload)?;
        let meta = PacketMeta {
            sequence,
            route_id: route_id.into(),
            generation,
            timestamp_ms,
            type_id,
            csid,
            stream_id,
            keyframe,
            configuration,
            size: payload.len(),
            offset: self.cursor,
        };
        self.cursor = end;
        self.sequence = sequence;
        self.used_bytes += payload.len() as u64;
        self.regions.insert(meta.offset, (sequence, meta.size));
        self.entries.push_back(meta.clone());
        self.insert_cache(sequence, Arc::from(payload));
        Ok(meta)
    }
    pub fn read(&mut self, sequence: u64) -> Result<StoredPacket, HistoryError> {
        let meta = self
            .entries
            .iter()
            .find(|p| p.sequence == sequence)
            .ok_or(HistoryError::Expired)?
            .clone();
        if let Some(payload) = self.cache.get(&sequence) {
            return Ok(StoredPacket {
                meta,
                payload: payload.clone(),
            });
        }
        let mut bytes = vec![0; meta.size];
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| std::io::Error::other("History is closed"))?;
        file.seek(SeekFrom::Start(meta.offset))?;
        file.read_exact(&mut bytes)?;
        let payload: Arc<[u8]> = bytes.into();
        self.insert_cache(sequence, payload.clone());
        Ok(StoredPacket { meta, payload })
    }
    pub fn seek_keyframe(
        &self,
        route_id: &str,
        generation: u64,
        target_ms: u64,
    ) -> Option<PacketMeta> {
        self.entries
            .iter()
            .rev()
            .find(|p| {
                p.route_id == route_id
                    && p.generation == generation
                    && p.keyframe
                    && !p.configuration
                    && p.timestamp_ms <= target_ms
            })
            .cloned()
    }
    pub fn first_keyframe_after(
        &self,
        route_id: &str,
        generation: u64,
        target_ms: u64,
    ) -> Option<PacketMeta> {
        self.entries
            .iter()
            .find(|p| {
                p.route_id == route_id
                    && p.generation == generation
                    && p.keyframe
                    && !p.configuration
                    && p.timestamp_ms >= target_ms
            })
            .cloned()
    }
    pub fn next(&self, route_id: &str, generation: u64, after_sequence: u64) -> Option<PacketMeta> {
        self.entries
            .iter()
            .find(|p| {
                p.route_id == route_id && p.generation == generation && p.sequence > after_sequence
            })
            .cloned()
    }
    pub fn oldest(&self, route_id: &str, generation: u64) -> Option<PacketMeta> {
        self.entries
            .iter()
            .find(|p| p.route_id == route_id && p.generation == generation)
            .cloned()
    }
    pub fn latest(&self, route_id: &str, generation: u64) -> Option<PacketMeta> {
        self.entries
            .iter()
            .rev()
            .find(|p| p.route_id == route_id && p.generation == generation)
            .cloned()
    }
    pub fn configurations(
        &self,
        route_id: &str,
        generation: u64,
        before_sequence: u64,
    ) -> Vec<PacketMeta> {
        let mut found = HashMap::new();
        for p in self.entries.iter().rev().filter(|p| {
            p.route_id == route_id
                && p.generation == generation
                && p.sequence <= before_sequence
                && p.configuration
        }) {
            found.entry(p.type_id).or_insert_with(|| p.clone());
        }
        let mut packets: Vec<_> = found.into_values().collect();
        packets.sort_by_key(|p| p.sequence);
        packets
    }
    pub fn route_bytes(&self, route_id: &str, generation: u64) -> u64 {
        self.entries
            .iter()
            .filter(|p| p.route_id == route_id && p.generation == generation)
            .map(|p| p.size as u64)
            .sum()
    }
    pub fn remove_route(&mut self, route_id: &str, generation: u64) {
        let remove: Vec<_> = self
            .entries
            .iter()
            .filter(|p| p.route_id == route_id && p.generation == generation)
            .map(|p| p.sequence)
            .collect();
        for sequence in remove {
            if let Some(p) = self.cache.remove(&sequence) {
                self.cache_bytes -= p.len();
            }
        }
        self.entries
            .retain(|p| p.route_id != route_id || p.generation != generation);
        let survivors: std::collections::HashSet<_> =
            self.entries.iter().map(|p| p.sequence).collect();
        self.regions.retain(|_, (seq, _)| survivors.contains(seq));
        self.cache_order.retain(|seq| self.cache.contains_key(seq));
        self.used_bytes = self.entries.iter().map(|p| p.size as u64).sum();
    }
    /// Release an expired prefix of one session without touching another route
    /// or the retained boundary packet. Codec contexts are pinned by the session.
    pub fn retain_from(&mut self, route_id: &str, generation: u64, first_sequence: u64) -> usize {
        if self
            .oldest(route_id, generation)
            .is_none_or(|p| p.sequence >= first_sequence)
        {
            return 0;
        }
        let before = self.entries.len();
        let mut removed = Vec::new();
        self.entries.retain(|packet| {
            let remove = packet.route_id == route_id
                && packet.generation == generation
                && packet.sequence < first_sequence;
            if remove {
                removed.push((packet.sequence, packet.offset, packet.size));
            }
            !remove
        });
        for (sequence, offset, size) in removed {
            self.used_bytes -= size as u64;
            self.regions.remove(&offset);
            if let Some(payload) = self.cache.remove(&sequence) {
                self.cache_bytes -= payload.len();
            }
        }
        self.cache_order
            .retain(|sequence| self.cache.contains_key(sequence));
        before - self.entries.len()
    }
    pub fn capacity(&self) -> u64 {
        self.capacity
    }
    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }
    pub fn cache_bytes(&self) -> usize {
        self.cache_bytes
    }
    pub fn packet_count(&self) -> usize {
        self.entries.len()
    }
    pub fn flush(&mut self) -> Result<(), HistoryError> {
        if let Some(file) = self.file.as_mut() {
            file.flush()?;
        }
        Ok(())
    }
    pub fn close(mut self) -> Result<(), HistoryError> {
        if let Some(file) = self.file.take() {
            file.sync_all()?;
            drop(file);
        }
        std::fs::remove_file(&self.path)?;
        self.path.clear();
        Ok(())
    }
    fn evict_front(&mut self) {
        if let Some(p) = self.entries.pop_front() {
            self.regions.remove(&p.offset);
            self.used_bytes -= p.size as u64;
            if let Some(bytes) = self.cache.remove(&p.sequence) {
                self.cache_bytes -= bytes.len();
            }
        }
        while self
            .cache_order
            .front()
            .is_some_and(|seq| !self.cache.contains_key(seq))
        {
            self.cache_order.pop_front();
        }
    }
    fn insert_cache(&mut self, sequence: u64, payload: Arc<[u8]>) {
        if payload.len() > self.cache_capacity {
            return;
        }
        while self.cache_bytes + payload.len() > self.cache_capacity {
            if let Some(seq) = self.cache_order.pop_front() {
                if let Some(bytes) = self.cache.remove(&seq) {
                    self.cache_bytes -= bytes.len();
                }
            } else {
                break;
            }
        }
        self.cache_bytes += payload.len();
        self.cache.insert(sequence, payload);
        self.cache_order.push_back(sequence);
    }
}
impl Drop for DiskHistory {
    fn drop(&mut self) {
        drop(self.file.take());
        if !self.path.as_os_str().is_empty() {
            if let Err(error) = std::fs::remove_file(&self.path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("Open-Delay history cleanup failed: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_retention_preserves_boundary_other_routes_and_generations() {
        let directory = tempfile::tempdir().unwrap();
        let mut h = DiskHistory::new(directory.path().join("ring"), 128, 128).unwrap();
        let expired = put(&mut h, "a", 0, b"old");
        let other = put(&mut h, "b", 0, b"other");
        let boundary = put(&mut h, "a", 1000, b"boundary");
        let future = put(&mut h, "a", 2000, b"future");
        let generation = h
            .append("a", 2, 0, 9, 6, 1, b"generation2", true, false)
            .unwrap();
        assert_eq!(h.retain_from("a", 1, boundary.sequence), 1);
        assert!(matches!(
            h.read(expired.sequence),
            Err(HistoryError::Expired)
        ));
        for (packet, payload) in [
            (other, b"other".as_slice()),
            (boundary, b"boundary".as_slice()),
            (future, b"future".as_slice()),
            (generation, b"generation2".as_slice()),
        ] {
            assert_eq!(&*h.read(packet.sequence).unwrap().payload, payload);
        }
        assert_eq!(h.used_bytes(), 5 + 8 + 6 + 11);
        assert_eq!(h.cache_bytes(), 30);
        assert_eq!(h.retain_from("a", 1, 1), 0);
    }
    fn put(h: &mut DiskHistory, route: &str, time: u64, payload: &[u8]) -> PacketMeta {
        h.append(route, 1, time, 9, 6, 1, payload, true, false)
            .unwrap()
    }
    #[test]
    fn wrap_expires_old_packets_without_corrupting_survivors() {
        let directory = tempfile::tempdir().unwrap();
        let mut h = DiskHistory::new(directory.path().join("ring"), 12, 4).unwrap();
        let a = put(&mut h, "a", 0, b"aaaa");
        let b = put(&mut h, "b", 1, b"bbbb");
        let c = put(&mut h, "a", 2, b"cccc");
        let d = put(&mut h, "a", 3, b"dddddd");
        assert!(matches!(h.read(a.sequence), Err(HistoryError::Expired)));
        assert!(matches!(h.read(b.sequence), Err(HistoryError::Expired)));
        assert_eq!(&*h.read(c.sequence).unwrap().payload, b"cccc");
        assert_eq!(&*h.read(d.sequence).unwrap().payload, b"dddddd");
        assert!(h.used_bytes() <= 12);
        assert!(h.cache_bytes() <= 4);
    }
    #[test]
    fn global_budget_is_shared_and_seek_is_route_scoped() {
        let directory = tempfile::tempdir().unwrap();
        let mut h = DiskHistory::new(directory.path().join("ring"), 16, 8).unwrap();
        let a = put(&mut h, "a", 10, b"aaaa");
        put(&mut h, "b", 20, b"bbbb");
        let c = put(&mut h, "a", 30, b"cccc");
        assert_eq!(h.seek_keyframe("a", 1, 25).unwrap().sequence, a.sequence);
        assert_eq!(h.next("a", 1, a.sequence).unwrap().sequence, c.sequence);
        assert!(h.seek_keyframe("a", 2, 100).is_none());
        assert_eq!(h.route_bytes("a", 1), 8);
        assert!(matches!(
            put_too_large(&mut h),
            Err(HistoryError::PacketTooLarge)
        ));
    }
    #[test]
    fn removed_routes_do_not_leave_overwritten_readable_packets() {
        let directory = tempfile::tempdir().unwrap();
        let mut h = DiskHistory::new(directory.path().join("ring"), 24, 0).unwrap();
        put(&mut h, "remove", 0, b"aaaaaaaa");
        let survivor = put(&mut h, "stay", 10, b"bbbbbbbb");
        put(&mut h, "stay", 20, b"cccccccc");
        h.remove_route("remove", 1);
        let fresh = put(&mut h, "stay", 30, b"dddddddddddd");
        assert!(matches!(
            h.read(survivor.sequence),
            Err(HistoryError::Expired)
        ));
        assert_eq!(&*h.read(fresh.sequence).unwrap().payload, b"dddddddddddd");
    }
    #[test]
    fn dropping_store_removes_only_its_owned_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ring");
        {
            let _h = DiskHistory::new(&path, 16, 0).unwrap();
            assert!(path.exists());
        }
        assert!(!path.exists());
        std::fs::write(&path, b"owned-by-someone-else").unwrap();
        assert!(DiskHistory::new(&path, 16, 0).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"owned-by-someone-else");
    }
    fn put_too_large(h: &mut DiskHistory) -> Result<PacketMeta, HistoryError> {
        h.append("a", 1, 0, 9, 6, 1, &[0; 17], true, false)
    }
}
