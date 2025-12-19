## TTorrent

I started this project because I wanted to learn Rust.

After watching a lot of YouTube videos, one common piece of advice I kept seeing was to build something on your own from scratch if you want to learn a language. Since I don't use Rust at work, I decided to follow this advice.

During the same period, I learned about Plex, which lets you build your own media system, and noticed that the majority of the files come from torrents. So, I decided to look for a torrent-related project and found a blog post about someone building their own torrent client in Go. I then decided to build my own in Rust.

I'm sure better implementations already exist, but I just wanted to learn something and have fun. Since torrents involve exchanging communication, it really appeals to me because it’s always awesome to receive a message from another computer on the network.

The main resources I’m using are this Go blog: https://blog.jse.li/posts/torrent/ and the "official" specification for torrents (I wasn't able to find an RFC): https://wiki.theory.org/BitTorrentSpecification#Tracker_HTTP.2FHTTPS_Protocol. For Rust, I am following the official book: https://doc.rust-lang.org/book/.

I am not just translating from Go to Rust; I am using the blog mainly because it is easy to follow and gives hints on the minimal stuff the protocol needs to work.

In order to actually learn the language, it doesn't make sense to use GenAI tools, so I am only asking questions regarding syntax and doing the implementation on my own—even if a better approach might exist.