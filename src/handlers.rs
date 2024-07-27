use std::collections::HashMap;
use crate::models::{ExportOptions, SharedAuthManager};
use clap::Parser;
use std::convert::Infallible;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use bytes::BytesMut;
use serde_json::Value;
use tokio::time::interval;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};
use tracing_subscriber::fmt::FormatFields;
use uuid::Uuid;
use warp::http::{Response, StatusCode};
use warp::Reply;
use warp::sse::Event;
use zip::{ZipWriter, write::FileOptions};
use crate::{Cli, Region};
use crate::claims::get_claims;
use crate::region::RegionWriter;

macro_rules! tri_resp {
    ($($t:tt)+) => {
        match $($t)+ {
            Ok(_) => {},
            Err(e) => return Ok(e.into_response())
        }
    }
}

// For hashing
#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq)]
struct Vec2i {
    x: i32,
    z: i32
}

fn add_anvil<W: Write + Seek>(zip: &mut ZipWriter<W>, chunks: &Vec<Vec<i32>>, target: &str, world: &PathBuf) -> Result<(), impl Reply> {
    let anvil_path = world.join(target);
    if !anvil_path.exists() {
        return Err(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(format!("region directory does not exist within world"))
            .into_response())
    }

    let mut loaded_regions = HashMap::new();

    let mut regions_out = HashMap::new();

    for coords in chunks {
        let x = coords.get(0);
        let z = coords.get(1);
        if x.is_none() || z.is_none() {
            return Err(Response::builder().status(StatusCode::BAD_REQUEST)
                .body("invalid coordinate provided")
                .into_response())
        }
        let x = *x.unwrap();
        let z = *z.unwrap();
        let region_x = x >> 5;
        let region_z = z >> 5;
        let vec2 = Vec2i { x: region_x, z: region_z };

        let region = match loaded_regions.get_mut(&vec2) {
            Some(mut r) => r,
            None => {
                let file_path = anvil_path.join(format!("r.{}.{}.mca", region_x, region_z));
                if !file_path.exists() {
                    // println!("skipping out-of-bounds region ({}, {})", region_x, region_z);
                    continue;
                }
                let f = File::open(file_path);
                if f.is_err() {
                    continue;
                }
                let f = f.unwrap();
                let r = Region::load(f);
                loaded_regions.insert(vec2, r);
                loaded_regions.get_mut(&vec2).unwrap()
            }
        };

        let mut out_region = match regions_out.get_mut(&vec2) {
            Some(mut or) => or,
            None => {
                let mut r = RegionWriter::new();
                regions_out.insert(vec2, r);
                regions_out.get_mut(&vec2).unwrap()
            }
        };

        let xmod = x % 32;
        let zmod = z % 32;
        let relative_x = if xmod < 0 { 32 + xmod } else { xmod };
        let relative_z = if zmod < 0 { 32 + zmod } else { zmod };

        if let Some(chunk) = region.get_chunk_raw(relative_x, relative_z) {
            out_region.set_chunk_raw(relative_x, relative_z, chunk);
        }
        out_region.set_chunk_timestamp(relative_x, relative_z,
                                       *region.get_timestamp(relative_x, relative_z)
                                           .unwrap_or(&0));
    }

    for (coords, region) in regions_out.iter() {
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.start_file(format!("{}/r.{}.{}.mca", target, coords.x, coords.z), options).unwrap();
        zip.write_all(&region.serialize()[..]).unwrap();
    }

    Ok(())
}

pub async fn export_chunks(opts: ExportOptions) -> Result<impl Reply, Infallible> {
    // check for world
    let dir = Cli::parse().path;
    let server_path = Path::new(&dir);
    if !server_path.exists() {
        return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR)
            .body("configured server directory does not exist").into_response())
    }

    let world_path = server_path.join(&opts.world);
    if !world_path.exists() {
        return Ok(Response::builder().status(StatusCode::BAD_REQUEST)
            .body("provided world does not exist").into_response())
    }

    let mut inner = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(inner);
    // Anvil data directories
    zip.add_directory("region/", FileOptions::default()).unwrap();
    zip.add_directory("entities/", FileOptions::default()).unwrap();
    zip.add_directory("poi/", FileOptions::default()).unwrap();

    tri_resp!(add_anvil(&mut zip, &opts.chunks, "region", &world_path));
    tri_resp!(add_anvil(&mut zip, &opts.chunks, "entities", &world_path));
    tri_resp!(add_anvil(&mut zip, &opts.chunks, "poi", &world_path));

    // level.dat
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);
    zip.start_file("level.dat", options).unwrap();
    zip.write_all(&match fs::read(world_path.join("level.dat")) {
        Ok(l) => l,
        Err(_) => {
            return Ok(Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR)
                .body("level.dat not found").into_response())
        }
    }[..]).unwrap();

    Ok(Response::builder().status(StatusCode::OK).body(zip.finish().unwrap().into_inner()).into_response())
}


pub async fn create_code(manager: SharedAuthManager) -> Result<impl Reply, Infallible> {
    let code = manager.lock().await.create_code();
    Ok(Response::builder().status(StatusCode::OK).body(code).into_response())
}

pub async fn poll_login(code: String, manager: SharedAuthManager) -> Result<impl Reply, Infallible> {
    let receiver = {
        match manager.lock().await.get_stream(&code) {
            Some(s) => s,
            None => return Ok(Response::builder().status(StatusCode::NOT_FOUND).body("Invalid one-time code!").into_response())
        }
    };

    let cancel = AtomicBool::new(false);

    let keepalive_stream = IntervalStream::new(interval(Duration::from_millis(2000))).map(move |_| {
        Ok::<Event, Infallible>(Event::default().comment("keepalive"))
    });

    let event_stream = ReceiverStream::new(receiver).map(move |v| {
        let data = if !cancel.load(Ordering::Relaxed) {
            if let Some(profile) = v {
                cancel.store(true, Ordering::Relaxed);
                let mut val = serde_json::to_value(&profile).unwrap();
                val.as_object_mut().unwrap().insert("claims".to_owned(), serde_json::to_value(&get_claims(Uuid::from_u128(0u128))).unwrap());
                serde_json::to_string(&val).unwrap()
            } else {
                "{}".to_owned()
            }
        } else {
            "{}".to_owned()
        };

        Ok::<Event, Infallible>(Event::default().data(data))
    });

    let stream = event_stream.merge(keepalive_stream);

    Ok(warp::sse::reply(stream).into_response())
}