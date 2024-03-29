#![crate_name = "router_os"]
#![crate_type = "lib"]

extern crate crypto;

use crypto::digest::Digest;
use crypto::md5::Md5;

use std::collections::BTreeMap;
use std::io::prelude::*;
use std::net::TcpStream;

fn hex_binascii<'a>(hexstr: String) -> Result<Vec<u8>, &'a str> {
    if hexstr.len() % 2 != 0 {
        Err("Odd number of characters")
    } else {
        let mut result: Vec<u8> = Vec::new();
        let mut i = 0;
        let c_hexstr: Vec<char> = hexstr.chars().collect();
        while i < c_hexstr.len() - 1 {
            let top = c_hexstr[i].to_digit(16).unwrap() as u8;
            let bottom = c_hexstr[i + 1].to_digit(16).unwrap() as u8;
            let r = (top << 4) + bottom;

            result.push(r);

            i += 2;
        }
        Ok(result)
    }
}

pub struct ApiRos<'a> {
    stream: &'a mut TcpStream,
}

impl<'a> ApiRos<'a> {
    pub fn new(s: &'a mut TcpStream) -> ApiRos<'a> {
        ApiRos { stream: s }
    }

    pub fn try_read(&mut self) -> bool {
        if self.stream.read(&mut [0]).unwrap() > 0 {
            true
        } else {
            false
        }
    }

    fn write_str(&mut self, str_buff: &[u8]) {
        match self.stream.write(str_buff) {
            Ok(_) => {}
            Err(e) => {
                panic!("connection closed by remote end, {}", e);
            }
        }
    }

    fn read_str(&mut self, length: usize) -> String {
        let mut buff: Vec<u8> = Vec::new();
        for _ in 0..length {
            let mut tmp_buff: [u8; 1] = [0];
            match self.stream.read(&mut tmp_buff) {
                Ok(_) => {}
                Err(e) => {
                    panic!("connection closed by remote end, {}", e);
                }
            }
            buff.push(tmp_buff[0]);
        }
        String::from_utf8(buff).unwrap()
    }

    fn write_len(&mut self, len: u32) {
        if len < 0x80 {
            self.write_str(&[len as u8]);
        } else if len < 0x4000 {
            let l = len | 0x8000;
            self.write_str(&[((l >> 8) & 0xFF) as u8]);
            self.write_str(&[(l & 0xFF) as u8]);
        } else if len < 0x200000 {
            let l = len | 0xC00000;
            self.write_str(&[((l >> 16) & 0xFF) as u8]);
            self.write_str(&[((l >> 8) & 0xFF) as u8]);
            self.write_str(&[(l & 0xFF) as u8]);
        } else if len < 0x10000000 {
            let l = len | 0xE0000000;
            self.write_str(&[((l >> 16) & 0xFF) as u8]);
            self.write_str(&[((l >> 24) & 0xFF) as u8]);
            self.write_str(&[((l >> 8) & 0xFF) as u8]);
            self.write_str(&[(l & 0xFF) as u8]);
        } else {
            self.write_str(&[(0xF0) as u8]);
            self.write_str(&[((len >> 24) & 0xFF) as u8]);
            self.write_str(&[((len >> 16) & 0xFF) as u8]);
            self.write_str(&[((len >> 8) & 0xFF) as u8]);
            self.write_str(&[(len & 0xFF) as u8]);
        }
    }

    fn read_len(&mut self) -> u32 {
        let mut c: u32 = self.read_str(1).as_bytes()[0] as u32;
        if c & 0x80 == 0x00 {
        } else if c & 0xC0 == 0x80 {
            c &= !0xC0;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
        } else if c & 0xE0 == 0xC0 {
            c &= !0xE0;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
        } else if c & 0xF0 == 0xE0 {
            c &= !0xF0;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
        } else if c & 0xF8 == 0xF0 {
            c = self.read_str(1).as_bytes()[0] as u32;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
            c <<= 8;
            c += self.read_str(1).as_bytes()[0] as u32;
            c += self.read_str(1).as_bytes()[0] as u32;
            c <<= 8;
        }
        c
    }

    fn read_word(&mut self) -> String {
        let len = self.read_len();
        let ret = self.read_str(len as usize);
//      if len > 0 {
//          println!(">>> {}", ret);
//      }
        ret
    }

    fn write_word(&mut self, w: String) {
//      if w.len() > 0 {
//          println!("<<< {}", w);
//      }
        self.write_len(w.len() as u32);
        self.write_str(&w.as_bytes());
    }

    pub fn write_sentence(&mut self, words: Vec<String>) -> u32 {
        let mut ret: u32 = 0;
        for w in words {
            self.write_word(w);
            ret += 1;
        }
        self.write_word("".to_string());
        ret
    }

    pub fn read_sentence(&mut self) -> Vec<String> {
        let mut r: Vec<String> = Vec::new();
        loop {
            let w = self.read_word();
            if &w[..] == "" {
                return r;
            }
            r.push(w);
        }
    }

    pub fn talk(
        &mut self,
        words: Vec<String>,
    ) -> Result<Vec<(String, BTreeMap<String, String>)>, &str> {
        if self.write_sentence(words) == 0 {
            return Ok(vec![]);
        }

        let mut r: Vec<(String, BTreeMap<String, String>)> = Vec::new();

        loop {
            let i: Vec<String> = self.read_sentence();
            if i.len() == 0 {
                continue;
            }

            let reply: String = i[0].clone();
            let mut attrs: BTreeMap<String, String> = BTreeMap::new();

            for w in &i[1..] {
                match w[1..].find("=") {
                    Some(n) => {
                        attrs.insert(w[..n + 1].to_string(), w[(n + 2)..].to_string());
                    }

                    None => {
                        attrs.insert(w.clone(), "".to_string());
                    }
                };
            }
            r.push((reply.clone(), attrs));
            if reply == "!done" {
                return Ok(r);
            } else if reply == "!trap" || reply == "!fatal" {
                return Err("API error");
            }
        }
    }

    pub fn login(&mut self, username: String, pwd: String) -> Result<(), &str> {
        match self.talk(vec![
            r"/login".to_string(),
            format!("=name={}", username),
            format!("=password={}", pwd),
        ]) {
            Ok(_reply) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
