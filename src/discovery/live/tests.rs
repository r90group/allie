use super::*;
use crate::test_support::write_live_discovery_manifest;
use std::io;
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use tempfile::tempdir;

#[test]
fn normalization_preserves_directory_paths_and_existing_query_fragment_policy() {
    for (input, expected) in [
        ("/", "/"),
        ("/docs", "/docs"),
        ("/docs/", "/docs/"),
        ("/docs/nested/", "/docs/nested/"),
        ("/docs/guide.html", "/docs/guide.html"),
        ("/docs/./", "/docs/"),
        ("/docs/.", "/docs/"),
        ("/docs/nested/..", "/docs/"),
        ("/docs/nested/../", "/docs/"),
        ("/docs/../", "/"),
        ("/docs/..", "/"),
        ("/docs/../guide.html", "/guide.html"),
        ("/docs/?lang=en#top", "/docs/"),
        ("/docs/guide.html?lang=en#top", "/docs/guide.html"),
        ("/docs?next=/", "/docs"),
        ("/docs#section/", "/docs"),
    ] {
        assert_eq!(normalize_live_route(input), expected, "input={input}");
    }
}

#[test]
fn link_resolution_keeps_directory_and_file_bases_distinct() {
    let base = LiveBaseUrl::parse("http://example.test:8080/docs/").unwrap();
    assert_eq!(base.start_route, "/docs/");
    assert_eq!(
        base.url_for_route(&base.start_route),
        "http://example.test:8080/docs/"
    );
    for (current, href, expected) in [
        ("/", "docs/", Some("/docs/")),
        ("/docs/", "guide.html", Some("/docs/guide.html")),
        ("/docs", "guide.html", Some("/guide.html")),
        ("/docs/index.html", "guide.html", Some("/docs/guide.html")),
        ("/docs/", "nested/", Some("/docs/nested/")),
        ("/docs/nested/", "../guide.html", Some("/docs/guide.html")),
        ("/docs/nested/", "..", Some("/docs/")),
        ("/docs/nested/", ".", Some("/docs/nested/")),
        ("/docs/", "../outside", Some("/outside")),
        ("/docs/", "guide.html?lang=en#top", Some("/docs/guide.html")),
        ("/docs/", "http://example.test:8080/api/", Some("/api/")),
        ("/docs/", "http://example.test:8081/api/", None),
        ("/docs/", "http://elsewhere.test:8080/api/", None),
        ("/docs/", "https://example.test:8080/api/", None),
        ("/docs/", "#top", None),
    ] {
        assert_eq!(
            resolve_live_link(&base, current, href).as_deref(),
            expected,
            "current={current}, href={href}"
        );
    }
}

#[test]
fn crawl_retains_distinct_slash_routes_without_changing_the_route_budget() {
    let site = DirectorySite::start();
    let discovery = discover_live_base_url_surfaces(&format!("{}/both", site.base_url)).unwrap();
    let routes = discovery
        .surfaces
        .iter()
        .map(|surface| surface.route.as_str())
        .collect::<BTreeSet<_>>();
    assert!(routes.contains("/docs"));
    assert!(routes.contains("/docs/"));
    assert!(routes.contains("/docs/guide.html"));
    assert!(
        discovery
            .surfaces
            .iter()
            .any(|surface| surface.route == "/docs" && surface.title == "File-like docs")
    );
    assert!(
        discovery
            .surfaces
            .iter()
            .any(|surface| surface.route == "/docs/" && surface.title == "Docs directory")
    );

    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    for index in 0..LIVE_DISCOVERY_MAX_PAGES {
        assert!(enqueue_route(
            &mut queue,
            &mut seen,
            format!("/page-{index}/")
        ));
    }
    assert!(enqueue_route(&mut queue, &mut seen, "/page-0/".to_string()));
    assert!(!enqueue_route(
        &mut queue,
        &mut seen,
        "/overflow/".to_string()
    ));
    assert_eq!(queue.len(), LIVE_DISCOVERY_MAX_PAGES);
}

#[test]
fn discovery_and_map_preserve_directory_routes_in_generated_artifacts() {
    let site = DirectorySite::start();
    for start in ["/", "/docs/"] {
        let temp = tempdir().unwrap();
        let manifest_path =
            write_live_discovery_manifest(temp.path(), &format!("{}{start}", site.base_url));
        let mut manifest = crate::FlowManifest::load(&manifest_path).unwrap();
        manifest.flow.states[0].id = "declared".to_string();
        manifest.flow.states[0].path = "/declared".to_string();
        std::fs::write(&manifest_path, serde_yaml::to_string(&manifest).unwrap()).unwrap();
        let project = temp.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let mut expected = BTreeSet::from([
            "/declared",
            "/docs/",
            "/docs/guide.html",
            "/docs/nested/",
            "/docs/nested/leaf.html",
            "/sitemap-dir/",
            "/sitemap-dir/leaf.html",
        ]);
        if start == "/" {
            expected.insert("/");
        }
        for command in ["discover", "map"] {
            let out = temp.path().join(command);
            let mut args = vec![
                command.to_string(),
                "--manifest".to_string(),
                manifest_path.display().to_string(),
                "--out".to_string(),
                out.display().to_string(),
            ];
            if command == "map" {
                args.extend(["--project-root".to_string(), project.display().to_string()]);
            }
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(
                crate::run_cli_with_io(args, &mut stdout, &mut stderr),
                0,
                "{}",
                String::from_utf8_lossy(&stderr)
            );
            let json = |name: &str| -> serde_json::Value {
                serde_json::from_str(&std::fs::read_to_string(out.join(name)).unwrap()).unwrap()
            };
            if command == "discover" {
                let packet = json("discovery.json");
                let routes = packet["surfaces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|surface| surface["route"].as_str().unwrap())
                    .collect::<BTreeSet<_>>();
                assert_eq!(routes, expected);
                assert!(packet["diagnostics"].as_array().unwrap().is_empty());
                let plan = json("flow-plan.json");
                let routes = plan["candidates"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|candidate| candidate["path"].as_str().unwrap())
                    .collect::<BTreeSet<_>>();
                assert_eq!(routes, expected);
                let directory = packet["surfaces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|surface| surface["route"] == "/docs/")
                    .unwrap();
                assert_eq!(
                    directory["provenance"][0],
                    format!("{}/docs/", site.base_url)
                );
            } else {
                let map = json("product-map.json");
                let routes = map["surfaces"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|surface| surface["routes"].as_array().unwrap())
                    .map(|route| route.as_str().unwrap())
                    .collect::<BTreeSet<_>>();
                assert_eq!(routes, expected);
                assert!(map["discovery_diagnostics"].as_array().unwrap().is_empty());
                let generated = crate::FlowManifest::load(&out.join("generated-flow.yml")).unwrap();
                let routes = generated
                    .flow
                    .states
                    .iter()
                    .map(|state| state.path.as_str())
                    .collect::<BTreeSet<_>>();
                assert_eq!(routes, expected);
            }
        }
    }
    let requests = site.requests.lock().unwrap();
    assert!(!requests.contains(&"/docs".to_string()));
    assert!(!requests.contains(&"/guide.html".to_string()));
    assert!(!requests.contains(&"/sitemap-dir".to_string()));
}

struct DirectorySite {
    base_url: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: mpsc::Sender<()>,
    handle: Option<thread::JoinHandle<()>>,
}

impl DirectorySite {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        let (stop, receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            loop {
                if receiver.try_recv().is_ok() {
                    break;
                }
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut bytes = [0; 4096];
                        let size = stream.read(&mut bytes).unwrap();
                        let request = String::from_utf8_lossy(&bytes[..size]);
                        let route = request
                            .lines()
                            .next()
                            .unwrap()
                            .split_whitespace()
                            .nth(1)
                            .unwrap();
                        recorded.lock().unwrap().push(route.to_string());
                        let (status, body) = match route {
                            "/" => ("200 OK", "<title>Root</title><a href='/docs/'>Docs</a>"),
                            "/both" => (
                                "200 OK",
                                "<a href='/docs'>File-like</a><a href='/docs/'>Directory</a>",
                            ),
                            "/docs" => ("200 OK", "<title>File-like docs</title>"),
                            "/docs/" => (
                                "200 OK",
                                "<title>Docs directory</title><a href='guide.html?lang=en#top'>Guide</a><a href='nested/'>Nested</a><a href='./'>Self</a>",
                            ),
                            "/docs/guide.html" => ("200 OK", "<title>Guide</title>"),
                            "/docs/nested/" => (
                                "200 OK",
                                "<title>Nested</title><a href='leaf.html'>Leaf</a><a href='..'>Parent</a>",
                            ),
                            "/docs/nested/leaf.html" => ("200 OK", "<title>Leaf</title>"),
                            "/sitemap.xml" => (
                                "200 OK",
                                "<urlset><url><loc>/sitemap-dir/</loc></url></urlset>",
                            ),
                            "/sitemap-dir/" => (
                                "200 OK",
                                "<title>Sitemap directory</title><a href='leaf.html'>Leaf</a>",
                            ),
                            "/sitemap-dir/leaf.html" => ("200 OK", "<title>Sitemap leaf</title>"),
                            _ => ("404 Not Found", "not found"),
                        };
                        let content_type = if route == "/sitemap.xml" {
                            "application/xml"
                        } else {
                            "text/html"
                        };
                        write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(error) => panic!("fixture listener failed: {error}"),
                }
            }
        });
        Self {
            base_url,
            requests,
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for DirectorySite {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }
}

mod ids;
