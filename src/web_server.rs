use warp::Filter;

pub async fn start_web_server(port: u16) {
    let web_dir = warp::fs::dir("web");
    
    println!("Starting web server on port {}", port);
    warp::serve(web_dir).run(([0, 0, 0, 0], port)).await;
}
