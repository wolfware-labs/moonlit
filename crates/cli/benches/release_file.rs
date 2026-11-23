use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use moonlit::release_file::{PluginEntry, add_plugin, set_plugin};
use moonlit_engine::pipeline::config::{FilesystemAccess, Permissions};

const RELEASE_YML: &str = include_str!("../../engine/benches/fixtures/release.yml");

fn add_release_plugin(c: &mut Criterion) {
  let entry = PluginEntry {
    name: "slack".to_string(),
    url: "oci://registry.moonlit.rs/wolfware/slack:1.0.0".to_string(),
    permissions: Permissions {
      network: vec!["slack.com".to_string()],
      exec: vec![],
      env: vec!["SLACK_*".to_string()],
      filesystem: FilesystemAccess::None,
    },
  };
  c.bench_function("add_plugin", |b| {
    b.iter(|| add_plugin(black_box(RELEASE_YML), black_box(&entry)).expect("plugin added"));
  });
}

fn set_release_plugin(c: &mut Criterion) {
  let permissions = Permissions {
    network: vec!["api.github.com".to_string(), "uploads.github.com".to_string()],
    exec: vec!["gh".to_string()],
    env: vec!["GITHUB_*".to_string()],
    filesystem: FilesystemAccess::ReadOnly,
  };
  c.bench_function("set_plugin", |b| {
    b.iter(|| {
      set_plugin(
        black_box(RELEASE_YML),
        "github",
        "oci://registry.moonlit.rs/wolfware/github:2.1.0",
        Some(black_box(&permissions)),
      )
      .expect("plugin updated")
    });
  });
}

criterion_group!(benches, add_release_plugin, set_release_plugin);
criterion_main!(benches);
