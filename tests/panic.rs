use mightybadger_test_server::sync::TestServer;
use std::thread;
use std::time::Duration;

#[test]
fn test_panic() {
    mightybadger::setup();
    let server = TestServer::new();
    let port = server.addr().port();
    mightybadger::configure(|config| {
        config.api_key = Some("abcdef".to_owned());
        config.connection.secure = Some(false);
        config.connection.host = Some("127.0.0.1".to_owned());
        config.connection.port = Some(port);
    });
    thread::sleep(Duration::from_millis(100));
    let th = thread::spawn(|| {
        panic!("panic test");
    });
    th.join().ok();
    {
        let data = server.data().read().unwrap();
        assert_eq!(data.errors.len(), 1);
    }
}
