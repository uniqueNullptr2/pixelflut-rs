use std::{sync::Arc, time::Duration, cmp::min};

use futures_util::{future::join_all};
use rand::Rng;
use tokio::{net::{TcpStream}, io::{copy, AsyncWriteExt, BufWriter}, time::sleep};
use std::path::PathBuf;
use itertools::Itertools;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Image {
        path: PathBuf,
        x: u32,
        y: u32,
        threads: usize
    },
    Nimages {
        path: PathBuf,
        threads: usize
    }
}


async fn n_images(path: &PathBuf, n: usize) {
    let img = image::open(path).unwrap().to_rgba8();
    let (w,h) = img.dimensions();
    

    let mut s = vec!();
    for _ in 0..n {
        let mut rng = rand::thread_rng();
        let dx = rng.gen_range(0..1920-w);
        let dy = rng.gen_range(0..1080-h);
        let pixels: Arc<Vec<String>> = Arc::new(img.enumerate_pixels().filter(|(_,_,px)| px.0[2] > 100).map(|(x, y, pix)| {
            format!("PX {} {} {:x}{:x}{:x}\n", x+dx, y+dy, pix.0[0], pix.0[1], pix.0[2])
        }).collect());
        s.push(tokio::spawn(async move {
            let addr = tokio::net::lookup_host("gpn-flut.poeschl.xyz:1234").await.unwrap().next().unwrap();
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut writer = BufWriter::new(stream);
            loop {
                for s in pixels.iter() {
                    writer.write_all(s.as_bytes()).await.unwrap();
                }
            }
        }));
    }
    
    join_all(s).await;
}

async fn image(path: &PathBuf, threads: usize, dx: u32, dy: u32) {
    let img = image::open(path).unwrap().to_rgba8();
    let pixels = Arc::new(img.enumerate_pixels().filter(|(_,_,px)| true).map(|(x, y, pix)| {
        format!("PX {} {} {:02x}{:02x}{:02x}\n", x+dx, y+dy, pix.0[0], pix.0[1], pix.0[2])
    }).collect::<Vec<String>>());
    let n = (pixels.len() as f64/threads as f64).ceil() as usize;
    let mut s = vec!();
    println!("{} pixels", pixels.len());
    for i in 0..threads {
        let p = pixels.clone();
        println!("thread {} goes from {} to {}", i, i*n, min(i*n+n, p.len()));
        s.push(tokio::spawn(async move {
            let addr = tokio::net::lookup_host("gpn-flut.poeschl.xyz:1234").await.unwrap().next().unwrap();
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let mut writer = BufWriter::new(stream);
            loop {
                for s in &p[i*n..min(i*n+n, p.len())] {
                    writer.write_all(s.as_bytes()).await.unwrap();
                }
            }
        }));
    }
    join_all(s).await;
}
#[tokio::main]
async fn main() {

    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Image { path, x, y, threads } => {
            image(path, *threads, *x, *y).await
        }
        Commands::Nimages { path, threads } => {
            n_images(path, *threads).await
        }
    }
    
}