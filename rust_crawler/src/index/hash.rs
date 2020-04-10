
use crate::index::fprocessing::eval;
use crate::index::lexem::Lexem;
use std::collections::HashMap;
use memmap::MmapOptions;
use std::io::{Write, Read};
use std::fs::{File, OpenOptions};

#[derive(Debug)]
pub struct HashBox {
    mem_pointer: u64,
    words_hash: u128,
    starts_with_hash: u128,
    ends_with_hash: u128,
    // hash4: u128,
    // hash5: u128,
    // hash6: u128,
}

impl HashBox {
    fn get_word_hash(hashable: &str) -> u128 {
        let mut xor_product = 0;
        hashable.chars().for_each(|x| xor_product ^= x as u32);
        let mut h = hashable.bytes();
        1u128 << (96 + hashable.len() % 16)
            | 1u128 << (112 + xor_product % 16)
            | 1u128 << (64 + ((h.next().unwrap() as u32 * 17) % 32))
            | 1u128 << ((h.next().unwrap() as u32 * 19) % 32)
            | 1u128 << (32 + (h.rev().next().unwrap() as u32 % 32))
    }
    fn get_starts_with_hash(hashable: &str) -> u128 {
        let mut h = hashable.bytes();
        let len = h.len();
        let mut result = 1u128 << (96 + (h.next().unwrap() as u32 % 32));
        if len > 1 {
            result |= 1u128 << (64 + (h.next().unwrap() as u32 % 32));
        }
        if len > 2 {
            result |= 1u128 << (32 + (h.next().unwrap() as u32 % 32));
        }
        if len > 3 {
            result |= 1u128 << (h.next().unwrap() as u32 % 32);
        }
        result
    }
    fn get_ends_with_hash(hashable: &str) -> u128 {
        let mut h = hashable.bytes().rev();
        let len = h.len();
        let mut result = 1u128 << (96 + (h.next_back().unwrap() as u32 % 32));
        if len > 1{
            result |= 1u128 << (64 + (h.next_back().unwrap() as u32 % 32));
        }
        if len > 2 {
            result |= 1u128 << (32 + (h.next_back().unwrap() as u32 % 32));
        }
        if len > 3 {
            result |= 1u128 << (h.next_back().unwrap() as u32 % 32);
        }
        result
    }
    
    pub fn match_ends_with(&self, h: &str) -> bool {
        let hash = HashBox::get_ends_with_hash(h);
        (self.ends_with_hash & hash) == hash
    }

    pub fn match_starts_with(&self, h: &str) -> bool {
        let hash = HashBox::get_starts_with_hash(h);
        (self.ends_with_hash & hash) == hash
    }

    pub fn get_ptr(&self) -> u64 {
        self.mem_pointer
    }

    pub fn size() -> usize {
        32
    }


    pub fn new(ptr:u64) -> Self {
        HashBox {
            mem_pointer: ptr,
            words_hash: 0,
            starts_with_hash: 0,
            ends_with_hash: 0,
            // hash4: 0,
            // hash5: 0,
            // hash6: 0,
        }
    }

    pub fn match_hashes(&self, hashable: &str) -> bool {
        let word = HashBox::get_word_hash(hashable);
        let start = HashBox::get_starts_with_hash(hashable);
        let end = HashBox::get_ends_with_hash(hashable);

        (self.words_hash & word) == word
            && (self.starts_with_hash & start) == start
            && (self.ends_with_hash & end) == end
    }

    pub fn add_hash(&mut self, hashable: &str) {
        self.words_hash |= HashBox::get_word_hash(hashable);
        self.starts_with_hash |= HashBox::get_starts_with_hash(hashable);
        self.ends_with_hash |= HashBox::get_ends_with_hash(hashable);
    }
}

#[derive(Debug)]
pub struct FileHasher {
    map: HashMap<String, Vec<HashBox>>,
}

impl FileHasher {
    pub fn new() -> Self {
        FileHasher {
            map: HashMap::new(),
        }
    }
    pub fn add(&mut self, v: Vec<HashBox>, k: &str) {
        self.map.insert(String::from(k), v);
    }

    pub fn look_out_hash<'a>(&'a mut self, lx: Lexem) -> Vec<String> {

        let mut result:Vec<String> = Vec::new();
        for item in self.map.iter() {
            if eval(&lx, &item.1[0], None) {
                let file = File::open(&format!("parsed/{}",item.0)).unwrap();
                let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
                for hbx in item.1[1..].iter() {
                    if eval(&lx, hbx, Some(&mmap)) {
                        result.push(item.0.to_string());
                        break;
                    }
                }
            }
        }
        result
    }

    pub fn write_to_file(
        &self,
        path : &str,
        keys : &Vec<String>,
    )
    {
        let mut file = File::create(path).unwrap();

        for key in keys {
            let item = self.map.get(key).unwrap();
            let usize_buf : [u8; 8] = unsafe {std::mem::transmute(item.len())};
            file.write(&usize_buf).expect("never fails");
            for hb in item {
                let bytes: [u8; 56] = unsafe {std::mem::transmute_copy(hb)};
                file.write(&bytes).expect("never fails");
            }
        }
    }

    pub fn read_from_stash(
        path: &str,
        links: Vec<String>
    ) -> Self {
        let mut hasher = FileHasher::new();
        let mut buf :[u8; 56] = [0;56];
        let mut usize_buf : [u8; 8] = [0;8];
        let mut ptr: usize = 0;
        let mut idx: usize = 0;
        match OpenOptions::new().read(true).open(path) {
            Ok(mut file) => {
                let mut bytes: Vec<u8> = Vec::new();
                file.read_to_end(&mut bytes).expect("can't read cache");
                while ptr < bytes.len() {
                    usize_buf.copy_from_slice(&bytes[ptr..ptr+8]);
                    let size : usize = unsafe {std::mem::transmute(usize_buf)};
                    ptr += 8;
                    let mut hash_vec : Vec<HashBox> = Vec::new(); 
                    for i in 0..size {
                        buf.copy_from_slice(&bytes[ptr..ptr+56]);
                        let hb : HashBox = unsafe {std::mem::transmute(buf)};
                        hash_vec.push(hb);
                        ptr+=56;
                    }
                    hasher.add(hash_vec, &links[idx]);
                    idx+=1;
                }
                //std::mem::transmute(e: T) 

            }
            Err(_) => {
            }
        }
        hasher
    }
}
