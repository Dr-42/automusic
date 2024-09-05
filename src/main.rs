/*
* Copyright (c) 2024, Dr. Spandan Roy
*
* This file is part of automusic.
*
* automusic is free software: you can redistribute it and/or modify
* it under the terms of the GNU General Public License as published by
* the Free Software Foundation, either version 3 of the License, or
* (at your option) any later version.
*
* automusic is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
* GNU General Public License for more details.
*
* You should have received a copy of the GNU General Public License
* along with automusic.  If not, see <https://www.gnu.org/licenses/>.
*/

use std::{io::Write, process::Child, thread::sleep, time::Duration};

use serde::{Deserialize, Serialize};
use sha256::digest;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BlockType {
    pub id: u8,
    pub name: String,
    pub color: Color,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum BlockId {
    Hobbies,
    Chill,
    Coding,
    Default,
}

fn play_mpv(music: &str, is_playlist: bool) -> std::process::Child {
    std::process::Command::new("mpv")
        .arg(music)
        .arg("-no-video")
        .arg("--shuffle")
        .spawn()
        .expect("Failed to play music")
}

fn main() {
    let cache_path = directories::ProjectDirs::from("org", "dr42", "automusic")
        .unwrap()
        .cache_dir()
        .to_owned();
    if !cache_path.exists() {
        std::fs::create_dir_all(&cache_path).unwrap();
    }
    let password_path = cache_path.join("password.txt");
    let server_ip_path = cache_path.join("server_ip.txt");

    let password = if let Ok(password) = std::fs::read_to_string(&password_path) {
        password
    } else {
        print!("Enter password: ");
        std::io::stdout().flush().unwrap();
        let mut password = String::new();
        std::io::stdin().read_line(&mut password).unwrap();
        let password = digest(password.trim());
        std::fs::write(&password_path, &password).unwrap();
        password
    };

    let server_ip = if let Ok(server_ip) = std::fs::read_to_string(&server_ip_path) {
        server_ip
    } else {
        print!("Enter server IP: ");
        std::io::stdout().flush().unwrap();
        let mut server_ip = String::new();
        std::io::stdin().read_line(&mut server_ip).unwrap();
        let server_ip = server_ip.trim();
        std::fs::write(&server_ip_path, server_ip).unwrap();
        server_ip.to_string()
    };

    // Use the password as auth header of Bearer token
    let block_types = reqwest::blocking::Client::new()
        .get(&format!("http://{}/blocktypes", server_ip))
        .header("Authorization", format!("Bearer {}", password));

    let block_types = block_types
        .send()
        .unwrap()
        .json::<Vec<BlockType>>()
        .unwrap();

    let hobbies_block_id = block_types
        .iter()
        .find(|block_type| block_type.name == "Hobbies")
        .unwrap()
        .id;
    let chill_block_id = block_types
        .iter()
        .find(|block_type| block_type.name == "Chill")
        .unwrap()
        .id;
    let coding_block_id = block_types
        .iter()
        .find(|block_type| block_type.name == "Coding")
        .unwrap()
        .id;

    let mut active_block_id = BlockId::Default;
    let mut active_process: Option<Child> = None;

    loop {
        let current_block_id = reqwest::blocking::Client::new()
            .get(&format!("http://{}/currentblocktype", server_ip))
            .header("Authorization", format!("Bearer {}", password));
        let current_block_id = current_block_id.send().unwrap().json::<u8>().unwrap();

        let block_id = if current_block_id == hobbies_block_id {
            BlockId::Hobbies
        } else if current_block_id == chill_block_id {
            BlockId::Chill
        } else if current_block_id == coding_block_id {
            BlockId::Coding
        } else {
            BlockId::Default
        };

        if active_block_id != block_id {
            active_block_id = block_id;

            // Play music based on block id
            let new_process = match active_block_id {
                BlockId::Hobbies => Some(play_mpv(
                    "https://music.youtube.com/playlist?list=PLeEm7S9XGtjijWUGbWKKkIDzEbc7X4PQx",
                    true,
                )),
                BlockId::Chill => Some(play_mpv(
                    "https://music.youtube.com/playlist?list=RDCLAK5uy_kb7EBi6y3GrtJri4_ZH56Ms786DFEimbM",
                    true,
                )),
                BlockId::Coding => Some(play_mpv(
                    "https://music.youtube.com/playlist?list=PL9LkJszkF_Z6bJ82689htd2wch-HVbzCO",
                    true,
                )),
                BlockId::Default => None,
            };
            if let Some(mut process) = active_process {
                process.kill().unwrap();
            }
            active_process = new_process;
        }

        sleep(Duration::from_secs(5));
    }
}
