use anyhow::{Context, Result};
use select::document::Document;
use select::node::Node;
use select::predicate::Class;
// use sxd_document::parser;
// use sxd_xpath::{evaluate_xpath, Value};

enum Currency {
    EUR,
    SEK,
}

#[tokio::main]
async fn main() -> Result<()> {
    use Currency::{EUR, SEK};

    let nordnet = vec![
        (
            "SBB NORDNET 16",
            275,
            "https://www.nordnet.se/marknaden/mini-futures/17380605-mini-l-sbb",
            SEK,
            10.98,
        ),
        (
            "INVESTOR NORDNET 40",
            7,
            "https://www.nordnet.se/marknaden/mini-futures/17459524-mini-l-investor",
            SEK,
            191.57,
        ),
        (
            "FACEBOOK NORDNET 17",
            44,
            "https://www.nordnet.se/marknaden/mini-futures/17268413-mini-l-facebook",
            SEK,
            41.09,
        ),
        (
            "ERICSSON NORDNET 40",
            77,
            "https://www.nordnet.se/marknaden/mini-futures/17232963-mini-l-ericsson",
            SEK,
            19.32,
        ),
        (
            "ERICSSON NORDNET 16",
            35,
            "https://www.nordnet.se/marknaden/mini-futures/16859714-mini-l-ericsson",
            SEK,
            28.89,
        ),
        (
            "BERKSHIRE NORDNET 02",
            24,
            "https://www.nordnet.se/marknaden/mini-futures/17601182-mini-l-berkshire",
            SEK,
            96.54,
        ),
        (
            "GOOGLE NORDNET F03",
            4,
            "https://www.nordnet.se/marknaden/mini-futures/17396890-long-google-nordnet",
            EUR,
            79.5,
        ),
    ];

    let mut portfolio_purchase_value = 0.0;
    let mut portfolio_sale_value = 0.0;

    for (name, amount, url, currency, gav) in nordnet {
        let res = reqwest::get(url).await?;
        let body = res.text().await?;
        let document = Document::from(body.as_str());
        let scrape: Vec<Node> = document.find(Class("bQbnak")).collect();
        let current_price = scrape
            .get(1)
            .context(format!("Did not find current price for {}", name))?
            .text()
            .replace(",", ".")
            .chars()
            .skip(4)
            .collect::<String>()
            .parse::<f64>()?;

        let base = match currency {
            SEK => 1.0,
            EUR => 10.4,
        };

        let current_price = current_price * base;
        let gav = gav * base;
        let purchase_value = gav * amount as f64;
        let sale_value = current_price * amount as f64;

        portfolio_purchase_value += purchase_value;
        portfolio_sale_value += sale_value;

        println!(
            "\n{}\nAmount: {}\nGAV: {:.2} (SEK)\nCurrent Price: {} (SEK)\nPurchase Value: {:.2} (SEK)",
            name, amount, gav, current_price, purchase_value
        );
    }

    println!(
        "\nPortfolio Purchase Value: {:.2} (SEK)",
        portfolio_purchase_value
    );

    println!(
        "Portfolio Sale Value:     {:.2} (SEK)",
        portfolio_sale_value
    );

    println!(
        "\nP/L: {:.2} (SEK)",
        portfolio_sale_value - portfolio_purchase_value
    );

    Ok(())
}
