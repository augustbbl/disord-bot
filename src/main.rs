#![feature(async_closure)]

use serde::Deserialize;

use serenity::{
    async_trait,
    client::{ Client, Context, EventHandler, },
    framework::standard::{
        StandardFramework,
        CommandResult,
        macros::{ command, group, }
    },
    model::{
        channel::Message,
        id::ChannelId,
    },
};

use std::{ error::Error, path::PathBuf, path::Path, };

type MyResult<T> = std::result::Result<T, Box<dyn Error>>;

use structopt::{
    clap,
    StructOpt,
};

#[derive(Deserialize)]
struct Config {
    token: String
}

impl Config {
    pub fn try_from_filepath<P: AsRef<Path>>(path: P) -> MyResult<Self> {
        use std::io::BufReader;
        use std::io::prelude::*;
        use std::fs::File;

        let file = File::open(path)?;
        let mut buf_reader = BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        Ok(toml::from_str(&contents)?)
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "zsh-bot")]
struct MainArgs {
    #[structopt(long, parse(from_os_str))]
    config: PathBuf,
}

#[group]
#[commands(ping, vote)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() -> MyResult<()> {
    let args = MainArgs::from_args();
    let config = Config::try_from_filepath(args.config)?;
    let framework = StandardFramework::new()
            .configure(|c| c.prefix("./"))
            .group(&GENERAL_GROUP);
    let mut client = Client::builder(config.token)
            .event_handler(Handler)
            .framework(framework)
            .await
            .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("client fucked up. {:?}", why);
    }

    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!").await?;
    Ok(())
}

#[derive(StructOpt, Debug)]
#[structopt(name = "vote")]
struct VoteArgs {
    #[structopt(short, long)]
    simple: bool,

    #[structopt(name = "PROPOSAL")]
    proposal: String,
}

fn parse_args<Args: StructOpt>(input: &str) -> Result<Args, clap::Error> {
    shell_words::split(input)
        .map_err(|e| clap::Error { 
            message: format!("Parsing Error: {}", e),  
            kind: clap::ErrorKind::Format,
            info: None,
        })
        .and_then(|words| Args::from_iter_safe(words))
}

const DEFAULT_REACT: [char; 5] = [
    '\u{2705}',  // White Heavy Check Mark Emoji
    '\u{1F1EB}', // Regional Indicator Symbol Letter F
    '\u{1F1F3}', // Regional Indicator Symbol Letter N
    '\u{1F1E6}', // Regional Indicator Symbol Letter A
    '\u{274E}',  // Negative Squared Cross Mark Emoji
];
const SIMPLE_REACT: [char; 2] = [
    '\u{2705}',  // White Heavy Check Mark Emoji
    '\u{274E}',  // Negative Squared Cross Mark Emoji
];

async fn simple_vote(args: VoteArgs, ctx: &Context, ch: &ChannelId) -> CommandResult {
    vote_impl(args, ctx, ch, &SIMPLE_REACT, 
              "\u{2705} - In Favor   \u{274E} - Against").await
}

async fn consensus(args: VoteArgs, ctx: &Context, ch: &ChannelId) -> CommandResult {
    vote_impl(args, ctx, ch, &DEFAULT_REACT, 
              "\u{2705} - Strongly In Favor  
               \u{1F1EB} - In Favor  
               \u{1F1F3} - Neutral
               \u{1F1E6} - Against
               \u{274E} - Strongly Against").await
}

async fn vote_impl(args: VoteArgs, ctx: &Context, ch: &ChannelId, 
                   reactions: &[char], vote_desc: &str) 
    -> CommandResult 
{
    let reply = ch.send_message(ctx, |m| {
        m.embed(|e| {
            e.title("PROPOSAL")
             .description(args.proposal)
             .field("Voting options:", vote_desc, false)
        })
    }).await?;
    for r in reactions {
        reply.react(ctx, *r).await?;
    }
    Ok(())
}

#[command]
async fn vote(ctx: &Context, msg: &Message) -> CommandResult {

    match parse_args::<VoteArgs>(&msg.content) {
        Ok(args) if args.simple == true 
                 => simple_vote(args, ctx, &msg.channel_id).await?,
        Ok(args) => consensus(args, ctx, &msg.channel_id).await?,
        Err(e) => { msg.reply(ctx, e).await?; },
    };
    Ok(())
}


