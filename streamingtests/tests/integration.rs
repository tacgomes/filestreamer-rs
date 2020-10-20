use std::cmp;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rand::prelude::*;
use serial_test::serial;
use sha2::{Digest, Sha256};

use filereceiver::FileReceiver;
use fileuploader::FileUploader;


fn create_test_file(filename: &str, size: usize) {
    let file = File::create(Path::new(filename)).unwrap();
    let mut writer = BufWriter::new(file);
    let mut rng = rand::thread_rng();
    let mut buffer = [0; 1024];
    let mut remaining = size;

    while remaining > 0 {
        let to_write = cmp::min(remaining, buffer.len());
        let buffer = &mut buffer[..to_write];
        rng.fill(buffer);
        writer.write(buffer).unwrap();
        remaining -= to_write;
    }
}

fn calculate_checksum(filename: &str) -> String {
    let mut file = File::open(Path::new(filename)).unwrap();
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).unwrap();
    format!("{:x}", hasher.finalize())
}

fn megabytes(n: usize) -> usize {
    n * 1024 * 1024
}

#[test]
#[serial]
fn test_streaming_basic() {
    let src_filename = "testfile10Mb";
    let dst_filename = &format!("{}.received", src_filename);

    create_test_file(src_filename, megabytes(10));

    let receiver = Arc::new(FileReceiver::new(8000));
    let receiver_clone = receiver.clone();

    let receiver_thread = thread::spawn(move || {
        receiver_clone.start();
    });

    let uploader_thread = thread::spawn(move || {
        let uploader = FileUploader::new("localhost".to_string(), 8000, 0);
        uploader.upload(src_filename.to_string());
    });

    uploader_thread.join().unwrap();

    receiver.stop();
    receiver_thread.join().unwrap();

    let checksum_original = calculate_checksum(src_filename);
    let checksum_copied = calculate_checksum(dst_filename);
    assert_eq!(checksum_original, checksum_copied);

    fs::remove_file(&src_filename).unwrap();
    fs::remove_file(&dst_filename).unwrap();
}

#[test]
#[serial]
fn test_streaming_restricted_upload_speed() {
    let src_filename = "testfile10Mb";
    let dst_filename = &format!("{}.received", src_filename);

    create_test_file(src_filename, megabytes(10));

    let receiver = Arc::new(FileReceiver::new(8000));
    let receiver_clone = receiver.clone();

    let receiver_thread = thread::spawn(move || {
        receiver_clone.start();
    });

    let uploader_thread = thread::spawn(move || {
        let uploader = FileUploader::new("localhost".to_string(), 8000, megabytes(1) as u32);
        uploader.upload(src_filename.to_string());
    });


    // The transfer should take 10 seconds to complete.
    // Allow a margin of error of 500 milliseconds.
    let now = Instant::now();
    uploader_thread.join().unwrap();
    let elapsed_millis = now.elapsed().as_millis();
    assert!(elapsed_millis > 9500 && elapsed_millis < 10500);

    receiver.stop();
    receiver_thread.join().unwrap();

    let checksum_original = calculate_checksum(src_filename);
    let checksum_copied = calculate_checksum(dst_filename);
    assert_eq!(checksum_original, checksum_copied);

    fs::remove_file(&src_filename).unwrap();
    fs::remove_file(&dst_filename).unwrap();
}

#[test]
#[serial]
fn test_streaming_resuming_upload() {
    let src_filename = "testfile10Mb";
    let dst_filename = &format!("{}.received", src_filename);

    create_test_file(src_filename, megabytes(10));

    let receiver = Arc::new(FileReceiver::new(8000));
    let receiver_clone_a = receiver.clone();
    let receiver_clone_b = receiver.clone();

    let receiver_thread = thread::spawn(move || {
        receiver_clone_a.start();
    });

    let uploader_thread = thread::spawn(move || {
        let uploader = FileUploader::new("localhost".to_string(), 8000, megabytes(1) as u32);
        uploader.upload(src_filename.to_string());
    });

    let now = Instant::now();

    thread::sleep(Duration::from_secs(5));
    receiver.stop_now();
    receiver_thread.join().unwrap();


    let receiver_thread = thread::spawn(move || {
        receiver_clone_b.start();
    });

    // The transfer should take a bit more than seconds to complete.
    uploader_thread.join().unwrap();
    let elapsed_millis = now.elapsed().as_millis();
    assert!(elapsed_millis > 10000 && elapsed_millis < 11500);

    receiver.stop();
    receiver_thread.join().unwrap();

    let checksum_original = calculate_checksum(src_filename);
    let checksum_copied = calculate_checksum(dst_filename);
    assert_eq!(checksum_original, checksum_copied);

    fs::remove_file(&src_filename).unwrap();
    fs::remove_file(&dst_filename).unwrap();
}
