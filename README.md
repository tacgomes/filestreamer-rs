Tool to upload a file to a remote machine.

Handles connectivity drops without having to reupload the whole file.

# Building

The project requires `rustc` and `cargo` to be installed in order to build it.

To build the project, run the following command:


```
cargo build
```

# Streaming a file

To stream a file, first start the receiver server:

```
./target/debug/file-receiver 8080
```

Now, in another terminal window, use the client to upload a file:

```
./target/debug/file-uploader --host 127.0.0.1 --port 8080 --limit-rate 1048576 testfile10Mb
```

Replace `testfile10Mb` with the file that you wish to upload. The file received
will have the `.received` suffix appended to its file name. The `--limit-rate`
parameter is optional and restricts the uploading speed to the given number of
bytes per second.
