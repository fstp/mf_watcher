use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local};
use futures::stream::{FuturesUnordered, StreamExt};
use job_scheduler::{Job, JobScheduler};
use select::document::Document;
use select::node::Node;
use select::predicate::Class;
use std::fmt;
use std::time::Duration;
use tokio::runtime::Runtime;
use std::rc::Rc;

enum Currency {
    EUR,
    SEK,
}

struct MfInfo {
    name: String,
    amount: i32,
    gav: f64,
    current_price: f64,
    purchase_value: f64,
    sale_value: f64,
}

impl fmt::Display for MfInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}\nAmount: {}\nGAV: {:.2} (SEK)\nCurrent Price: {:.2} (SEK)\nPurchase Value: {:.2} (SEK)\nSale Value: {:.2}",
            self.name, self.amount, self.gav, self.current_price, self.purchase_value, self.sale_value
        )
    }
}

async fn process_nordnet(
    name: &str,
    amount: i32,
    url: &str,
    currency: Currency,
    gav: f64,
) -> Result<MfInfo> {
    use Currency::{EUR, SEK};

    let res = reqwest::get(url).await?;
    let body = res.text().await?;
    let document = Document::from(body.as_str());
    let scrape: Vec<Node> = document.find(Class("bQbnak")).collect();

    let current_price = {
        let b = scrape
            .get(0)
            .context(format!("Unable to scrape buy price for {}", name))?
            .text();

        let s = scrape
            .get(1)
            .context(format!("Unable to scrape sale price for {}", name))?
            .text();

        let f = |x: String, len: usize| -> Result<f64> {
            x.replace(",", ".")
                .chars()
                .skip(len)
                .collect::<String>()
                .parse::<f64>()
                .context(format!("Failed to parse {}", x))
        };

        let b = f(b, 3)?;
        let s = f(s, 4)?;

        if s <= 0.0 && b <= 0.0 {
            bail!("No price found for {}, skipping", name);
        }

        if s <= 0.0 {
            b
        } else {
            s
        }
    };

    let base = match currency {
        SEK => 1.0,
        EUR => 10.4,
    };

    let current_price = current_price * base;
    let gav = gav * base;
    let purchase_value = gav * amount as f64;
    let sale_value = current_price * amount as f64;

    // portfolio_purchase_value += purchase_value;
    // portfolio_sale_value += sale_value;

    // println!(
    //     "\n{}\nAmount: {}\nGAV: {:.2} (SEK)\nCurrent Price: {:.2} (SEK)\nPurchase Value: {:.2} (SEK)\nSale Value: {:.2}",
    //     name, amount, gav, current_price, purchase_value, sale_value
    // );

    Ok(MfInfo {
        name: name.to_owned(),
        amount,
        gav,
        current_price,
        purchase_value,
        sale_value,
    })
}

async fn scrape_minifutures() -> Result<()> {
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

    let mut stream = FuturesUnordered::new();

    for (name, amount, url, currency, gav) in nordnet {
        stream.push(process_nordnet(name, amount, url, currency, gav));
    }

    loop {
        match stream.next().await {
            Some(Ok(mf)) => {
                push_to_es(&mf);
                println!("\n{}", mf);
                portfolio_purchase_value += mf.purchase_value;
                portfolio_sale_value += mf.sale_value;
            }
            Some(Err(e)) => {
                println!("\n{}", e)
            }
            None => {
                break;
            }
        }
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



    let local: DateTime<Local> = Local::now();
    println!("\n{}", local.format("%A %Y-%m-%d %H:%M:%S"));

    Ok(())
}

async fn push_to_es(mf: &MfInfo) -> Result<()> {
    Ok(())
}

//#[tokio::main]
fn main() -> Result<()> {
    let mut sched = JobScheduler::new();
    let rt = Rc::new(Runtime::new().unwrap());

    run_task(rt.clone())?;

    sched.add(Job::new(
        "1/30 * * * * *".parse().unwrap(),
        move || match run_task(rt.clone()) {
            Ok(_) => println!("\nScheduling next run...\n"),
            Err(e) => println!("\n{:?}\n", e),
        },
    ));

    loop {
        sched.tick();
        std::thread::sleep(Duration::from_millis(500));
    }
}

fn run_task(rt: Rc<Runtime>) -> Result<()> {
    rt.block_on(scrape_minifutures())
}
