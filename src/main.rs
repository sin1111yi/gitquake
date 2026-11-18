mod cli;
mod config;
mod git;
mod model;
mod plain;
mod rows;
mod stats;
mod theme;
mod ui;
mod util;

use std::io::IsTerminal;

use anyhow::Result;
use clap::Parser;

use cli::Cli;
use config::HeatScale;
use git::LoadOptions;
use theme::Quality;
use ui::app::{App, ReloadCtx};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let loaded = config::load(cli.config.as_deref(), cli.no_config);
    let mut cfg = loaded.config;
    let mut warnings = std::mem::take(&mut cfg.warnings);

    if let Some(theme) = &cli.theme {
        cfg.general.theme = theme.clone();
    }
    if let Some(scale) = &cli.scale {
        match HeatScale::parse(scale) {
            Some(v) => cfg.general.heat_scale = v,
            None => warnings.push(format!("unknown scale '{scale}', using log")),
        }
    }
    if cli.all {
        cfg.general.all_branches = true;
    }

    if cli.dump_config {
        print!("{}", cfg.dump());
        return Ok(());
    }
    if cli.list_themes {
        plain::list_themes(&cfg);
        return Ok(());
    }

    let quality = Quality::detect();
    let theme = theme::resolve(&cfg, &mut warnings);

    let limit = cli.limit.unwrap_or(cfg.general.limit);
    let opts = LoadOptions {
        all: cfg.general.all_branches,
        limit,
        since: cli.since.clone(),
        author: cli.author.clone(),
        no_merges: cli.no_merges,
    };

    let repo = git::load(&cli.path, &opts)?;
    let reload_ctx = ReloadCtx {
        config_path: cli.config.clone(),
        no_config: cli.no_config,
        theme: cli.theme.clone(),
        scale: cli.scale.clone(),
        limit: cli.limit,
    };
    let mut app = App::new(repo, cfg, theme, quality, opts, reload_ctx, warnings);

    let stdout_tty = std::io::stdout().is_terminal();
    let color = !cli.no_color && (stdout_tty || cli.color);
    if cli.plain || !stdout_tty {
        plain::print(&app, color)
    } else {
        ui::run(&mut app)
    }
}
