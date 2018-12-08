## Test-Game
#### A text-based game and "learn Rust" project with a few interesting goals

Test-Game is a text-based game designed with a few major goals in mind:
- Procedural map generation, however basic. 
- Procedural item generation.
- Time-based events.
- Modability; items and area types are registered statically to allow for the creation of a public API for adding new content to the game.
- Online multiplayer, including Discord integration and a dedicated client / server interface.
- The possibility of converting the game into a 2D dungeon-crawler, although this likely will not happen unless a third party is willing to help.

Beyond that, I had a few personal goals with this project:
- First and foremost, to learn Rust. This means that project was started when I had almost no knowledge of the language, outside of a small, 200-300 line demo I wrote beforehand.
- To improve my programming skills by exploring new challenges and pushing myself to do things I hadn't done in prior projects. I'm an amature programmer with less than 10-11 months of experience of coding in my free time only and learning a language with stricter guidelines than Java, JavaScript, or Kotlin is an important milestone in my own development.

Initially, this project was intended to span no more than 4,000-5,000 LOC; however, I wound up developing certain aspects of the game to a level that I had not anticipated. At the time of this writing, we're at nearly 12k LOC, including the main project and its dependencies, which are also available here on GitHub. As such, it's really unclear how much further this project will develop. I have some more ideas for content in the game, but content is not my primary concern. I'm mostly worried about creating a rudimentary engine which can be used to *produce* new content.

#### What's working

Currently, Test-Game supports conditional compilation to allow for use of the Discord bot and or dedicated server. There is a possibility that the Discord bot is not currently working. I have changed some elements of the game which it depends on, but have not attempted to maintain it, due to its slow rate of responding to queries from clients. This problem is more-so related to Discord and not the game, and thus any attempt to remedy these problems does not really make all that much sense; however, this was initially intended to be a sort of fancy Discord bot, so that feature will not go away completely. On the other hand, a dedicated client has been written and is fully functional. The only thing it's missing is support for passwords, but this will not be needed unless save games are added.

#### How to play

The basic gameplay arc at the moment is like this: 
- Go through a very basic character creation dialogue.
- Walk around, visit, and test some early areas, including a pub, a travel station, usually a fountain, sometimes a gambling den, an altar, etc.
- Gain temporary buffs in each town, if you can find a fountain and afford to make enough donations.
- Gain one permanent buff and one permanent debuff at each altar. If the altar is a monument to the god you worship, you do not get a debuff.
- Speak to and test some basic trade mechanics with NPCs. If you and the NPC both worship the same god, you'll get access to its special trades. These are currently just swords (mostly working) and bows (dummies), but are planned to be lower-tier, unbreakable items.
- Wave to other players who have connected remotely.

...And that's it. There are supposed to bosses and enemies, but I still need to finish a few more things before those are ready to be implemented.

##### Here are some global commands you can use:
- `settings <open>` to open the settings menu and optionally have it stay open.
- `players` to view a list of all currently-active players.
- `msg <username>` to send a message to another player. Currently does nothing.

If you have access to the server, you also get these commands:
- `pause` | `p` to pause or unpause the game.
- `end` or `quit` to close the game.

If you're running a dedicated client, you can also use this command:
- `end` | `quit` | `leave` | `stop` to disconnect from the host.

If cheats are enabled, you can also use these commands:
- `tp [<town#> | <type>]` to teleport around. This will not display entrance messages.
  - `town#` represents the number of the town to travel to.
  - `type` represents the type of area. These are currently `altar`, `boss`, `dungeon`, `fountain`, `gambling`, `gate`, `shop`, and `station`.
- `money #` to give yourself `#` gold.
- `god x` to change your god to `x`.

#### Etc.

Beyond that, please feel free to do nothing more than criticize my code. That's why this project exists! If you have suggestions for code improvements or would even like to contribute new content, please submit an issue here on GitHub. 
