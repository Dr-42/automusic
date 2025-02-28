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
mod netutils;

use std::{collections::HashMap, io::Write, process::Child, thread::sleep, time::Duration};

use blockconfig::BlockConfig;
use netutils::{LoginRequest, LoginResponse, Meta};
use reqwest::blocking::Client;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentBlock {
    pub block_type_id: u8,
    pub current_block_name: String,
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

    let data_path = directories::ProjectDirs::from("org", "dr42", "automusic")
        .unwrap()
        .data_dir()
        .to_owned();
    if !data_path.exists() {
        std::fs::create_dir_all(&data_path).unwrap();
    }
    let meta_path = data_path.join("meta.json");

    let _ = if let Ok(meta) = std::fs::read_to_string(&meta_path) {
        serde_json::from_str(&meta).unwrap()
    } else {
        println!("Please enter the server_ip");
        let server_ip = std::io::stdin().lines().next().unwrap().unwrap();
        println!("Please enter the password");
        let password = rpassword::read_password().unwrap();
        let hashed_pass = digest(&password);
        let login_req = LoginRequest { key: hashed_pass };
        let client = Client::new();
        let response = client
            .post(format!("http://{}/auth/login", server_ip))
            .json(&login_req)
            .send()
            .unwrap()
            .json::<LoginResponse>()
            .unwrap();

        let meta = Meta {
            server_ip: server_ip.to_string(),
            access_token: response.access_token,
            refresh_token: response.refresh_token,
        };

        std::fs::write(meta_path, serde_json::to_string(&meta).unwrap()).unwrap();

        meta
    };

    let block_types = loop {
        // let block_types = reqwest::blocking::Client::new()
        //     .get(format!("http://{}/blocktype/get", meta.server_ip))
        //     .header("Authorization", format!("Bearer {}", password));
        let block_types =
            netutils::make_get_request::<Vec<BlockType>>("/blocktype/get", &data_path, None);

        if block_types.is_err() {
            sleep(Duration::from_secs(5));
            continue;
        }
        break block_types.unwrap();
    };

    let mut id_map: HashMap<u8, Vec<BlockConfig>> =
        blockconfigs
            .clone()
            .into_iter()
            .fold(HashMap::new(), |mut map, block_config| {
                let block_type = block_types
                    .iter()
                    .find(|block_type| block_type.name.trim() == block_config.type_name.trim());

                if block_type.is_none() {
                    eprintln!("Block type {} not found", block_config.type_name);
                    return map;
                }
                let block_type = block_type.unwrap();
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

        let current_block =
            netutils::make_get_request::<CurrentBlock>("/currentblock/get", &data_path, None);
        if current_block.is_err() {
            sleep(Duration::from_secs(5));
            continue;
        }
        let current_block = current_block.unwrap();
        let current_block_id = current_block.block_type_id;
        let current_block_name = current_block.current_block_name;

        if active_block_id != current_block_id || active_block_name != current_block_name {
            active_block_id = current_block_id;
            active_block_name = current_block_name;

            let new_process = id_map.get(&active_block_id).and_then(|block_configs| {
                block_configs
                    .iter()
                    .find(|block_config| {
                        block_config.block_name.as_ref() == Some(&active_block_name)
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
