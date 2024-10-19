use litemap::LiteMap;
use async_io::Timer;

use crate::{DATABASE, FULL_DB_ACCESS, Receiver, or};
use crate::database::objects::Hash;

use std::fs::{write, rename, remove_file};
use std::time::{Instant, Duration};
use std::process::exit;
use std::mem::take;

const BACKUP_PERIOD_MINUTES: u64 = 20;

fn try_remove_file(hash: &Hash) {
    let path = format!("files/{}.dat", hash);
    if let Err(error) = remove_file(path) {
        println!("FGC: Failed to delete file {}: {:?}", hash, error);
    } else {
        println!("FGC: Removed {}", hash)
    }
}

pub async fn backup_task(rx_signal: Receiver<()>) {
    loop {
        let timeout = async {
            Timer::after(Duration::from_secs(60 * BACKUP_PERIOD_MINUTES)).await;
            false
        };

        let recv_save_signal = async {
            rx_signal.recv().await.unwrap();
            true
        };

        let must_exit = or(timeout, recv_save_signal).await;

        let then = Instant::now();
        println!("scheduled database backup");
        let _writer = FULL_DB_ACCESS.write().await;
        println!("beginning database backup");

        // file garbage collection
        if true {
            let mut file_rc = DATABASE.file_rc.write().await;
            let mut filtered = Vec::with_capacity(file_rc.len());

            for (hash, counter) in take(&mut *file_rc).into_tuple_vec() {
                match counter {
                    0 => try_remove_file(&hash),
                    _ => filtered.push((hash, counter)),
                }
            }

            *file_rc = LiteMap::from_sorted_store_unchecked(filtered);
        }

        rename("database.json", "database-old.json").unwrap();
        let json_dump = serde_json::to_string_pretty(&DATABASE).unwrap();
        write("database.json", json_dump.as_bytes()).unwrap();

        let duration = then.elapsed().as_millis();
        println!("database backed up in {}ms", duration);
        if must_exit {
            exit(0);
        }
    }
}