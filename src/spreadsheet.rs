use crate::comparator::{Action, CompareResult, CompareStatus};
use crate::i18n::t;
use anyhow::Result;
use chrono::Local;

pub enum ExportFormat {
    Xlsx,
    Csv,
}

/// Generate a timestamped filename next to the executable.
/// e.g. packages_report_20260429_214501.xlsx
pub fn auto_output_path(format: &ExportFormat) -> String {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let ext = match format {
        ExportFormat::Xlsx => "xlsx",
        ExportFormat::Csv => "csv",
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    exe_dir
        .join(format!("packages_report_{}.{}", ts, ext))
        .to_string_lossy()
        .to_string()
}

pub fn export(
    results: &[CompareResult],
    path: &str,
    format: ExportFormat,
) -> Result<()> {
    match format {
        ExportFormat::Xlsx => export_xlsx(results, path),
        ExportFormat::Csv => export_csv(results, path),
    }
}

fn status_label(status: &CompareStatus) -> String {
    match status {
        CompareStatus::UpstreamNewer => t("status_upstream_newer"),
        CompareStatus::UpstreamOnly => t("status_upstream_only"),
        CompareStatus::MineOnly => t("status_mine_only"),
        CompareStatus::Same => t("status_same"),
        CompareStatus::Manual => t("status_manual"),
    }
}

fn action_label(action: &Action) -> String {
    match action {
        Action::Overwrite => t("action_overwrite"),
        Action::Add => t("action_add"),
        Action::Keep => t("action_keep"),
        Action::Skip => t("action_skip"),
    }
}

fn export_xlsx(results: &[CompareResult], path: &str) -> Result<()> {
    use rust_xlsxwriter::*;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White);

    let headers = [
        t("col_name"),
        t("col_mine_ver"),
        t("col_upstream_ver"),
        t("col_status"),
        t("col_action"),
        t("col_path"),
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet.write_with_format(0, col as u16, header.as_str(), &header_format)?;
    }

    let row_colors = [
        (CompareStatus::UpstreamNewer, Color::RGB(0xFFEB9C)),
        (CompareStatus::UpstreamOnly, Color::RGB(0xC6EFCE)),
        (CompareStatus::MineOnly, Color::RGB(0xE2EFDA)),
        (CompareStatus::Manual, Color::RGB(0xFFC7CE)),
    ];

    for (row, result) in results.iter().enumerate() {
        let row = row as u32 + 1;
        let bg_color = row_colors
            .iter()
            .find(|(s, _)| s == &result.status)
            .map(|(_, c)| *c)
            .unwrap_or(Color::White);
        let fmt = Format::new().set_background_color(bg_color);

        sheet.write_with_format(row, 0, &result.name, &fmt)?;
        sheet.write_with_format(row, 1, result.my_version(), &fmt)?;
        sheet.write_with_format(row, 2, result.upstream_version(), &fmt)?;
        sheet.write_with_format(row, 3, status_label(&result.status).as_str(), &fmt)?;
        sheet.write_with_format(row, 4, action_label(&result.action).as_str(), &fmt)?;
        let display_path = if result.mine.is_some() {
            result.my_path()
        } else {
            result.upstream_path()
        };
        sheet.write_with_format(row, 5, display_path, &fmt)?;
    }

    sheet.autofit();
    workbook.save(path)?;
    Ok(())
}

fn export_csv(results: &[CompareResult], path: &str) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;

    wtr.write_record([
        t("col_name").as_str(),
        t("col_mine_ver").as_str(),
        t("col_upstream_ver").as_str(),
        t("col_status").as_str(),
        t("col_action").as_str(),
        t("col_path").as_str(),
    ])?;

    for result in results {
        let display_path = if result.mine.is_some() {
            result.my_path()
        } else {
            result.upstream_path()
        };
        wtr.write_record([
            &result.name,
            result.my_version(),
            result.upstream_version(),
            &status_label(&result.status),
            &action_label(&result.action),
            display_path,
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
