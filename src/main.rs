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

mod blockconfig;

use std::{collections::HashMap, io::Write, process::Child, thread::sleep, time::Duration};

use blockconfig::BlockConfig;
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

fn play_mpv(music: &str, is_playlist: bool) -> std::process::Child {
    if is_playlist {
        std::process::Command::new("mpv")
            .arg(music)
            .arg("-no-video")
            .arg("--shuffle")
            .arg("--loop-playlist")
            .spawn()
            .expect("Failed to play music")
    } else {
        std::process::Command::new("mpv")
            .arg(music)
            .arg("-no-video")
            .arg("--loop")
            .spawn()
            .expect("Failed to play music")
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("{} - {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        return;
    }
    if std::env::args().any(|arg| arg == "--help") {
        println!("Automatically play music based on the current block type");
        println!();
        println!("USAGE:");
        println!("    {} [FLAGS]", env!("CARGO_PKG_NAME"));
        println!();
        println!("FLAGS:");
        println!("    --version    Prints version information");
        println!("    --help       Prints help information");
        return;
    }

    let mut blockconfigs = BlockConfig::getall();
    let mut last_update = BlockConfig::get_last_update();

    if std::env::args().any(|arg| arg == "add") {
        let mut input = String::new();
        print!("Enter the block type name: ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        let type_name = input.clone().trim().to_string();
        print!("Enter the block name: (* for all) ");
        std::io::stdout().flush().unwrap();
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        let block_name = if input.trim() == "*" {
            None
        } else {
            Some(input.trim().to_string())
        };

        // Check if this config already exists
        if blockconfigs.iter().any(|block_config| {
            block_config.type_name == type_name && block_config.block_name == block_name
        }) {
            println!("Config already exists");
            println!(
                "Config: {}",
                blockconfigs
                    .iter()
                    .find(|block_config| {
                        block_config.type_name == type_name && block_config.block_name == block_name
                    })
                    .unwrap()
            );
            return;
        }
        print!("Enter the music URL: ");
        std::io::stdout().flush().unwrap();
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        let music_url = input.trim().to_string();
        print!("Is it a playlist? (y/n) ");
        std::io::stdout().flush().unwrap();
        input.clear();
        std::io::stdin().read_line(&mut input).unwrap();
        let is_playlist = input.trim() == "y";

        let block_config =
            blockconfig::BlockConfig::new(type_name, block_name, music_url, is_playlist);
        block_config.add_block();
        return;
    }

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
        .get(format!("http://{}/blocktypes", server_ip))
        .header("Authorization", format!("Bearer {}", password));

    let block_types = block_types
        .send()
        .unwrap()
        .json::<Vec<BlockType>>()
        .unwrap();

    let mut id_map: HashMap<u8, Vec<BlockConfig>> =
        blockconfigs
            .clone()
            .into_iter()
            .fold(HashMap::new(), |mut map, block_config| {
                let block_type = block_types
                    .iter()
                    .find(|block_type| block_type.name == block_config.type_name)
                    .unwrap();
                map.entry(block_type.id).or_default().push(block_config);
                map
            });

    let mut active_block_id = 255;
    let mut active_block_name = String::new();
    let mut active_process: Option<Child> = None;

    loop {
        // Check if block configs has been updated
        if BlockConfig::check_update(last_update) {
            last_update = BlockConfig::get_last_update();
            blockconfigs = blockconfig::BlockConfig::getall();
            id_map =
                blockconfigs
                    .clone()
                    .into_iter()
                    .fold(HashMap::new(), |mut map, block_config| {
                        let block_type = block_types
                            .iter()
                            .find(|block_type| block_type.name == block_config.type_name)
                            .unwrap();
                        map.entry(block_type.id).or_default().push(block_config);
                        map
                    });
        }
        let current_block_id = reqwest::blocking::Client::new()
            .get(format!("http://{}/currentblocktype", server_ip))
            .header("Authorization", format!("Bearer {}", password));
        let current_block_name = reqwest::blocking::Client::new()
            .get(format!("http://{}/currentblockname", server_ip))
            .header("Authorization", format!("Bearer {}", password));
        let current_block_id = current_block_id.send().unwrap().json::<u8>();
        if current_block_id.is_err() {
            sleep(Duration::from_secs(5));
            continue;
        }
        let current_block_name = current_block_name.send().unwrap().json::<String>();
        if current_block_name.is_err() {
            sleep(Duration::from_secs(5));
            continue;
        }
        let current_block_id = current_block_id.unwrap();
        let current_block_name = current_block_name.unwrap();

        if active_block_id != current_block_id || active_block_name != current_block_name {
            active_block_id = current_block_id;
            active_block_name = current_block_name;

            let new_process = id_map.get(&active_block_id).and_then(|block_configs| {
                block_configs
                    .iter()
                    .find(|block_config| {
                        block_config
                            .block_name
                            .as_ref()
                            .map_or(false, |block_name| block_name == &active_block_name)
                    })
                    .or_else(|| {
                        block_configs
                            .iter()
                            .find(|block_config| block_config.block_name.is_none())
                    })
                    .map(|block_config| play_mpv(&block_config.music_url, block_config.is_playlist))
            });

            if let Some(mut process) = active_process {
                process.kill().unwrap();
            }
            active_process = new_process;
        }

        sleep(Duration::from_secs(5));
    }
}
