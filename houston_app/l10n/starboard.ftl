### Text resources for the starboard.

# When the user requests a page beyond what there is.
no-page-found = No data for this page.

# When the first page is already empty.
no-content = <None>

post = { $link }: { $max-reacts } { $emoji }
post-by-user = { $link } by { $user }: { $max-reacts } { $emoji }
user-score = { $user }: { $score } { $emoji } from { $post-count } { $post-count ->
        [1] post
       *[other] posts
    }

# Texts for the `/starboard top-posts` command.
top-posts =
    .header = { $emoji } Top Posts
    .by-user-header = By: { $user }

top =
    .header = { $emoji } Leaderboards

overview =
    .header = Starboard Overview
    .top-post = Top Post
    .top-poster = Top Poster

error-not-enabled = Starboard is not enabled for this server.
error-unknown = Unknown Starboard.
