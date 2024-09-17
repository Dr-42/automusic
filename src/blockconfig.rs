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

use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BlockConfig {
    pub type_name: String,
    pub block_name: Option<String>,
    pub music_url: String,
    pub is_playlist: bool,
}

impl Display for BlockConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl BlockConfig {
    pub fn new(
        type_name: String,
        block_name: Option<String>,
        music_url: String,
        is_playlist: bool,
    ) -> Self {
        Self {
            type_name,
            block_name,
            music_url,
            is_playlist,
        }
    }

    pub fn getall() -> Vec<BlockConfig> {
        let config_dir = directories::ProjectDirs::from("org", "dr42", "automusic")
            .unwrap()
            .config_dir()
            .to_owned();
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).unwrap();
        }
        let config_path = config_dir.join("config.json");
        if !config_path.exists() {
            std::fs::write(&config_path, "[]").unwrap();
        }
        serde_json::from_str(&std::fs::read_to_string(config_path).unwrap()).unwrap()
    }

    pub fn add_block(&self) {
        let mut blocks = Self::getall();
        blocks.push(self.clone());
        let config_dir = directories::ProjectDirs::from("org", "dr42", "automusic")
            .unwrap()
            .config_dir()
            .to_owned();
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).unwrap();
        }
        let config_path = config_dir.join("config.json");
        if !config_path.exists() {
            std::fs::write(&config_path, "[]").unwrap();
        }

        std::fs::write(config_path, serde_json::to_string_pretty(&blocks).unwrap()).unwrap();
    }
}
