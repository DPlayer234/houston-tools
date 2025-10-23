### Text resources for the starboard.

# When the user requests a page beyond what there is.
no-page-found = No data for this page.

# When the first page is already empty.
no-page-content = <None>

# Texts for the `/starboard top-posts` command.
top-posts =
    .header = { $emoji } Top Posts
    .by-user-header = By: { $user }
    .entry = { $rank }. { $link }: { $max-reacts } { $emoji }
    .entry-by-user = { $rank }. { $link } by { $user }: { $max-reacts } { $emoji }

top =
    .header = { $emoji } Leaderboards
    .entry = { $rank }. { $user }: { $score } { $emoji } from { $post-count } { $post-count ->
        [1] post
       *[other] posts
    }
