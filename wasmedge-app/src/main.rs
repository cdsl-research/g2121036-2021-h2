use bytecodec::{bytes::BytesEncoder, io::IoEncodeExt, DecodeExt, Encode};
use httpcodec::{
    BodyEncoder, HttpVersion, ReasonPhrase, Request, RequestDecoder, Response, ResponseEncoder,
    StatusCode,
};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};
use wasmedge_wasi_socket::{Shutdown, TcpListener, TcpStream};

fn file_server(path: &str) -> Option<Vec<u8>> {
    let mut target = PathBuf::from(urlencoding::decode(path).unwrap().into_owned());

    if target.is_dir() {
        target.push("index.html");
    }

    if target.exists() && target.is_file() {
        let mut f = File::open(target.as_path()).unwrap();
        let mut buf = vec![];
        f.read_to_end(&mut buf).unwrap();
        Some(buf)
    } else {
        None
    }
}


// /// streamからHTTPリクエストのヘッダーとボディの全てを受けとって返す
// fn _recv_all(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
//     let mut buff = [0u8; 1024];
//     let mut data = Vec::new();

//     loop {
//         let n = stream.read(&mut buff)?;
//         println!("DEBUG: revc {} byte", n);
//         data.extend_from_slice(&buff[..n]);

//         let mut headers = [httparse::EMPTY_HEADER; 16];
//         let mut req = httparse::Request::new(&mut headers);

//         match req.parse(&data).unwrap() {
//             Status::Complete(body_offset) => {
//                 let content_length: usize =
//                     if let Some(h) = req.headers.iter().find(|h| h.name.eq("Content-Length")) {
//                         std::str::from_utf8(h.value).unwrap().parse().unwrap()
//                     } else {
//                         break;
//                     };

//                 if content_length <= data.len() - body_offset {
//                     break;
//                 }
//             }
//             Status::Partial => {
//                 println!("partial");
//                 break;
//             }
//         }
//     }

//     Ok(data)
// }

fn handle_http(req: Request<String>) -> bytecodec::Result<Response<Vec<u8>>> {
    match file_server(req.request_target().as_str()) {
        Some(v) => Ok(Response::new(
            HttpVersion::V1_0,
            StatusCode::new(200)?,
            ReasonPhrase::new("")?,
            v,
        )),
        None => Ok(Response::new(
            HttpVersion::V1_0,
            StatusCode::new(404)?,
            ReasonPhrase::new("Not found")?,
            "404 NOT FOUND".into(),
        )),
    }
}

fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buff = [0u8; 1024];
    let mut data = Vec::new();

    loop {
        let n = stream.read(&mut buff)?;
        data.extend_from_slice(&buff[0..n]);
        if n < 1024 {
            break;
        }
    }

    let mut decoder =
        RequestDecoder::<httpcodec::BodyDecoder<bytecodec::bytes::Utf8Decoder>>::default();

    let req = match decoder.decode_from_bytes(data.as_slice()) {
        Ok(req) => handle_http(req),
        Err(e) => Err(e),
    };

    let r = match req {
        Ok(r) => r,
        Err(e) => {
            let err = format!("{:?}", e);
            Response::new(
                HttpVersion::V1_0,
                StatusCode::new(500).unwrap(),
                ReasonPhrase::new(&err.to_string()).unwrap(),
                err.into_bytes(),
            )
        }
    };

    let mut encoder = ResponseEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    encoder.start_encoding(r).unwrap();
    let mut data = Vec::new();
    encoder.encode_all(&mut data).unwrap();

    stream.write(&data)?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or("1234".to_string());
    println!("new connection at {}", port);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port), false)?;
    loop {
        let _ = handle_client(listener.accept(false)?.0);
    }
}
