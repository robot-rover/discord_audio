use std::fs;

use ::serenity::{async_trait, http::CacheHttp};
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use songbird::{Event, EventContext, TrackEvent, events::{EventHandler}, typemap::TypeMapKey, input::{cached::{Compressed, Memory}, File}};

// https://discord.com/api/oauth2/authorize?client_id=184445488093724672&permissions=8&scope=bot%20applications.commands

struct Data {} // User data, which is stored and accessible in all command invocations
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

struct TrackErrorNotifier;
struct AudioFileType;
impl TypeMapKey for AudioFileType {
    type Value = Memory;
}

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

/// Displays your or another user's account creation date
#[poise::command(slash_command)]
async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("https://discord.com/api/oauth2/authorize?client_id=184445488093724672&permissions=8&scope=bot%20applications.commands").await?;
    Ok(())
}

#[poise::command(slash_command)]
async fn join(ctx: Context<'_>,
    #[description = "Information about a server voice channel"]
    #[channel_types("Voice")]
    channel: serenity::GuildChannel) -> Result<(), Error> {
    let http = ctx.serenity_context().http();
    println!("Joining {} {:?} {:?}", ctx.author().id, ctx.guild_channel().await.map(|gc| gc.id), ctx.author_member().await.map(|am| am.joined_at));
    // let channel_id = ctx.guild_channel().await.ok_or("Must be in a server").and_then(|guild_channel| {
    //     let gid = ctx.guild_id().ok_or("Must be in a server");
    //     println!("Test {:?}", guild_channel.guild(ctx.cache()).unwrap().voice_states);
    //     // let test = ctx.author_member().await.unwrap();
    //     Ok(())
    //     // guild_channel
    //     //     .voice_states
    //     //     .get(&ctx.author().id)
    //     //     .and_then(|vs| vs.channel_id.map(|vid| (gid, vid)))
    //     //     .ok_or("You are not in a voice channel")
    // });

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice Client not initialized")
        .clone();

    let handler_lock = manager
        .join(channel.guild_id, channel.id)
        .await
        .expect("Couldn't join voice channel");

    let rw_lock = ctx.serenity_context().data.read().await;
    let audio = rw_lock.get::<AudioFileType>().unwrap();

    let mut handler = handler_lock.lock().await;
    handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
    handler.play_input(audio.new_handle().into());

    ctx.reply("Connected!").await.unwrap();

    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![join()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let audio = Memory::new(File::new("bloom.mp3").into()).await.unwrap();
    // let audio = Compressed::new(File::new("bloom.mp3").into(), songbird::driver::Bitrate::Auto).await.unwrap();
    let client =
        songbird::register(serenity::ClientBuilder::new(token, intents).framework(framework).type_map_insert::<AudioFileType>(audio)).await;
    client.unwrap().start().await.unwrap();
}
