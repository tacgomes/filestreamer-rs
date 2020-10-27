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

use file_receiver::FileReceiver;
use file_uploader::FileUploader;

const SERVER_PORT: u16 = 8080;

fn create_test_file<P: AsRef<Path>>(file_name: P, size: usize) {
    let file = File::create(file_name).unwrap();
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

fn calculate_checksum<P: AsRef<Path>>(file_name: P) -> String {
    let mut file = File::open(file_name).unwrap();
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
    let src_file_name = "testfile10Mb";
    let dst_file_name = &format!("{}.received", src_file_name);

    create_test_file(src_file_name, megabytes(10));

    let receiver = Arc::new(FileReceiver::new(SERVER_PORT));
    let receiver_clone = receiver.clone();

    let receiver_thread = thread::spawn(move || {
        receiver_clone.start();
    });

    let uploader_thread = thread::spawn(move || {
        let uploader = FileUploader::new(
            "localhost".to_string(),
            SERVER_PORT,
            None
        );
        uploader.upload(src_file_name.to_string());
    });

    uploader_thread.join().unwrap();

    receiver.stop();
    receiver_thread.join().unwrap();

    let checksum_original = calculate_checksum(src_file_name);
    let checksum_copied = calculate_checksum(dst_file_name);

    fs::remove_file(&src_file_name).unwrap();
    fs::remove_file(&dst_file_name).unwrap();

    assert_eq!(checksum_original, checksum_copied);
}

#[test]
#[serial]
fn test_streaming_restricted_upload_speed() {
    let src_file_name = "testfile10Mb";
    let dst_file_name = &format!("{}.received", src_file_name);

    create_test_file(src_file_name, megabytes(10));

    let receiver = Arc::new(FileReceiver::new(SERVER_PORT));
    let receiver_clone = receiver.clone();

    let receiver_thread = thread::spawn(move || {
        receiver_clone.start();
    });

    let uploader_thread = thread::spawn(move || {
        let uploader = FileUploader::new(
            "localhost".to_string(),
            SERVER_PORT,
            Some(megabytes(1) as u32),
        );
        uploader.upload(src_file_name.to_string());
    });

    let now = Instant::now();
    uploader_thread.join().unwrap();
    let elapsed_millis = now.elapsed().as_millis();

    receiver.stop();
    receiver_thread.join().unwrap();

    let checksum_original = calculate_checksum(src_file_name);
    let checksum_copied = calculate_checksum(dst_file_name);
    fs::remove_file(&src_file_name).unwrap();
    fs::remove_file(&dst_file_name).unwrap();

    // The transfer should take 10 seconds to complete. Allow some
    // margin of error necessary for the tests for pass in the CI.
    assert!(elapsed_millis > 9500 && elapsed_millis < 11500);

    assert_eq!(checksum_original, checksum_copied);
}

#[test]
#[serial]
fn test_streaming_resuming_upload() {
    let src_file_name = "testfile10Mb";
    let dst_file_name = &format!("{}.received", src_file_name);

    create_test_file(src_file_name, megabytes(10));

    let receiver = Arc::new(FileReceiver::new(SERVER_PORT));
    let receiver_clone_a = receiver.clone();
    let receiver_clone_b = receiver.clone();

    let receiver_thread = thread::spawn(move || {
        receiver_clone_a.start();
    });

    let uploader_thread = thread::spawn(move || {
        let uploader = FileUploader::new(
            "localhost".to_string(),
            SERVER_PORT,
            Some(megabytes(1) as u32),
        );
        uploader.upload(src_file_name.to_string());
    });

    let now = Instant::now();

    thread::sleep(Duration::from_secs(5));
    receiver.stop_now();
    receiver_thread.join().unwrap();

    let receiver_thread = thread::spawn(move || {
        receiver_clone_b.start();
    });

    uploader_thread.join().unwrap();
    let elapsed_millis = now.elapsed().as_millis();

    receiver.stop();
    receiver_thread.join().unwrap();

    let checksum_original = calculate_checksum(src_file_name);
    let checksum_copied = calculate_checksum(dst_file_name);

    fs::remove_file(&src_file_name).unwrap();
    fs::remove_file(&dst_file_name).unwrap();

    // The transfer should take a bit more than 10 seconds to complete.
    // Allow some margin of error necessary for the tests for pass in
    // the CI.
    assert!(elapsed_millis > 10000 && elapsed_millis < 12500);

    assert_eq!(checksum_original, checksum_copied);
}
