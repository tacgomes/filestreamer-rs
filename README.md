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
./target/debug/filereceiver 8080
```

Now, in another terminal window, use the client to upload a file:

```
./target/debug/fileuploader --host 127.0.0.1 --port 8080 testfile10Mb
```

Replace `testfile10Mb` with the file that you wish to upload. 
