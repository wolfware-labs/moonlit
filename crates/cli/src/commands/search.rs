use comfy_table::{ContentLineStyle, LineStyle, Table, TableStyle};
use console::style;
use moonlit_engine::plugin::publish::PLUGIN_WORLD;
use moonlit_engine::registry::{MiddlewareSummary, PackageHit, RegistryClient, RegistryError, VerificationStatus};

use crate::cli::{DEFAULT_REGISTRY_HOST, OutputMode, SearchArgs};
use crate::commands::common::fail;
use crate::display::{badge, display_safe, status_key};
use crate::render::resolve_mode;

const TABLE_STYLE: TableStyle = TableStyle::new()
  .top_border(LineStyle::new('┌', '─', '┬', '┐'))
  .header_lines(ContentLineStyle::new('│', '│', '│'))
  .header_separator(LineStyle::new('├', '─', '┼', '┤'))
  .content_lines(ContentLineStyle::new('│', '│', '│'))
  .bottom_border(LineStyle::new('└', '─', '┴', '┘'));

const INCOMPATIBLE_MARKER: &str = "(not compatible with this moonlit; add picks the newest compatible version)";

struct Card {
  hit: PackageHit,
  middlewares: Vec<MiddlewareSummary>,
  compatible: Option<bool>,
}

#[must_use]
pub async fn run(output: Option<OutputMode>, args: SearchArgs) -> i32 {
  match search(output, &args).await {
    Ok(code) | Err(code) => code,
  }
}

async fn search(output: Option<OutputMode>, args: &SearchArgs) -> Result<i32, i32> {
  let registry = args.registry.as_deref().unwrap_or(DEFAULT_REGISTRY_HOST);
  let client = RegistryClient::for_host(registry).or_else(|e| fail(&e.to_string(), 2))?;
  let cards = fetch(&client, args).await.or_else(|e| fail(&e.to_string(), 3))?;
  match resolve_mode(output) {
    OutputMode::Json => print_json(&cards),
    mode => print_cards(&cards, &args.query, mode == OutputMode::Pretty),
  }
  Ok(0)
}

async fn fetch(client: &RegistryClient, args: &SearchArgs) -> Result<Vec<Card>, RegistryError> {
  let hits = client.search(&args.query, args.limit).await?;
  let mut cards = Vec::with_capacity(hits.len());
  for hit in hits {
    let detail = match &hit.latest_version {
      Some(version) => client.package_version(&hit.org, &hit.name, version).await?,
      None => None,
    };
    let compatible = detail
      .as_ref()
      .map(|detail| detail.world_version.as_deref() == Some(PLUGIN_WORLD));
    let middlewares = detail.map(|detail| detail.middlewares).unwrap_or_default();
    cards.push(Card {
      hit,
      middlewares,
      compatible,
    });
  }
  Ok(cards)
}

fn print_json(cards: &[Card]) {
  let arr: Vec<_> = cards
    .iter()
    .map(|c| {
      serde_json::json!({
          "org": c.hit.org,
          "name": c.hit.name,
          "version": c.hit.latest_version,
          "description": c.hit.description,
          "compatible": c.compatible,
          "status": status_key(c.hit.status),
          "middlewares": c.middlewares.iter().map(|m| serde_json::json!({
              "name": m.name,
              "description": m.description,
          })).collect::<Vec<_>>(),
      })
    })
    .collect();
  println!("{}", serde_json::to_string_pretty(&arr).unwrap());
}

fn print_cards(cards: &[Card], query: &str, pretty: bool) {
  let Some(first) = cards.first() else {
    println!("No plugins match '{}'.", display_safe(query));
    return;
  };
  let count = cards.len();
  println!("{count} plugin{}", if count == 1 { "" } else { "s" });
  for card in cards {
    println!();
    let hit = &card.hit;
    let id = display_safe(&format!("{}/{}", hit.org, hit.name));
    let title = hit
      .latest_version
      .as_deref()
      .map_or_else(|| id.clone(), |v| format!("{id} {}", display_safe(v)));
    let marker = (card.compatible == Some(false)).then_some(INCOMPATIBLE_MARKER);
    let badge = badge(hit.status);
    if pretty {
      let badge = match hit.status {
        VerificationStatus::Verified => style(badge).green(),
        VerificationStatus::Failed => style(badge).red(),
        _ => style(badge).dim(),
      };
      let marker = marker.map_or_else(String::new, |m| format!(" {}", style(m).dim()));
      println!("{}{marker}   {badge}", style(title).bold());
    } else {
      let marker = marker.map_or_else(String::new, |m| format!(" {m}"));
      println!("{title}{marker}   {badge}");
    }
    if let Some(description) = &hit.description {
      println!("{}", display_safe(description));
    }
    if !card.middlewares.is_empty() {
      let mut table = Table::new();
      table.load_style(TABLE_STYLE);
      table.set_header(["MIDDLEWARE", "DESCRIPTION"]);
      for m in &card.middlewares {
        table.add_row([display_safe(&m.name), display_safe(m.description.as_deref().unwrap_or(""))]);
      }
      for line in table.to_string().lines() {
        println!("  {line}");
      }
    }
  }
  println!();
  println!(
    "Add one: moonlit add {}",
    display_safe(&format!("{}/{}", first.hit.org, first.hit.name))
  );
}
