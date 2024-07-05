mod mongobar;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Mongobar::new("qxg")
    //     .clean()
    //     .op_record((
    //         DateTime::parse_rfc3339_str("2024-07-03T10:54:18.837Z").unwrap(),
    //         DateTime::parse_rfc3339_str("2024-07-05T10:54:18.838Z").unwrap(),
    //     ))
    //     .await?;
    mongobar::Mongobar::new("qxg").init().op_stress().await?;

    Ok(())
}
