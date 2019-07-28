use actix_web::{App, HttpServer};

//fn index(req: HttpRequest) -> Result<HttpResponse> {
//    // response
//    Ok(HttpResponse::build(StatusCode::OK)
//        .content_type("text/html; charset=utf-8")
//        .body(include_str!("web_server/index.html")))
//}

pub fn start_web_server() {
    std::thread::spawn(move || {
        let sys = actix_rt::System::new("rustarok");

        HttpServer::new(|| {
            App::new()
                // static files
                .service(
                    actix_files::Files::new("/", "web_server")
                        .index_file("index.html")
                )
        })
            .workers(1)
            .bind("0.0.0.0:6868")?
            .start();

        log::info!("Starting http server: 0.0.0.0:6868");
        sys.run()
    });
}