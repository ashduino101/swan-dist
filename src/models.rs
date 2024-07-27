use std::collections::HashMap;
use std::sync::Arc;
use rand::distributions::DistString;
use rand::prelude::Distribution;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::mpsc::{Sender, Receiver};
use tokio::sync::{mpsc, Mutex};
use crate::Profile;

// Query params for an export request
#[derive(Debug, Deserialize)]
pub struct ExportOptions {
    pub world: String,
    pub chunks: Vec<Vec<i32>>,
}

pub type SharedAuthManager = Arc<Mutex<AuthManager>>;

struct CodeDist;

impl Distribution<u8> for CodeDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        const RANGE: u32 = 26 + 10;  // lowercase letters + numbers
        const GEN_ASCII_STR_CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

        loop {
            // We could *almost* get away with 32-5 bits here, but we're 4 characters over,
            // so we still have to shift by 32-6
            let var = rng.next_u32() >> (32 - 6);
            if var < RANGE {
                return GEN_ASCII_STR_CHARSET[var as usize];
            }
        }
    }
}

impl DistString for CodeDist {
    fn append_string<R: Rng + ?Sized>(&self, rng: &mut R, string: &mut String, len: usize) {
        unsafe {
            let v = string.as_mut_vec();
            v.extend(self.sample_iter(rng).take(len));
        }
    }
}

#[derive(Debug, Clone)]
pub struct OneTimeCode {
    pub(crate) used: bool,
    pub(crate) sender: Sender<Option<Profile>>
}

impl OneTimeCode {
    pub fn new() -> OneTimeCode {
        OneTimeCode {
            used: false,
            sender: mpsc::channel(1).0  // placeholder
        }
    }

    pub fn get_stream(&mut self) -> Receiver<Option<Profile>> {
        let (sender, receiver) = mpsc::channel(4);
        self.sender = sender;
        receiver
    }

    pub fn invalidate(&mut self) {
        self.used = true;
    }
}

#[derive(Debug, Clone)]
pub struct AuthManager {
    one_time_codes: HashMap<String, OneTimeCode>
}

impl AuthManager {
    pub fn new() -> AuthManager {
        AuthManager {
            one_time_codes: HashMap::new()
        }
    }

    pub fn create_code(&mut self) -> String {
        let code = CodeDist.sample_string(&mut rand::thread_rng(), 16);
        self.one_time_codes.insert(code.clone(), OneTimeCode::new());
        code
    }

    pub fn has_code(&self, code: &String) -> bool {
        self.one_time_codes.contains_key(code)
    }

    pub fn is_code_used(&self, code: &String) -> bool {
        self.has_code(code) && self.one_time_codes.get(code).unwrap().used
    }

    pub fn use_code(&mut self, code: &String) -> Option<()> {
        self.one_time_codes.get_mut(code)?.invalidate();
        Some(())
    }

    pub fn get_stream(&mut self, code: &String) -> Option<Receiver<Option<Profile>>> {
        Some(self.one_time_codes.get_mut(code)?.get_stream())
    }

    pub fn get_sender(&mut self, code: &String) -> Option<Sender<Option<Profile>>> {
        Some(self.one_time_codes.get(code)?.sender.clone())
    }
}
