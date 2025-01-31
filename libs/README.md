# Houston Tools Library Crates

Versioned via semver, so you _could_ depend on them.
Except `azur_lane` and `houston_cmd`. I consider these to be somewhat internal so these _may_ be a bit funky in terms of versioning.

Do however note that versions are only incremented when there's changes after a "release".

Please also refer to the documentation in the crate if you intend to use these.

## `azur_lane`

Partial data model for Azur Lane game data. This is essentially the data the Azur Lane Data Collector collects.

## `houston_cmd`

Custom serenity slash-command only framework.

Unlike poise, this has been designed exclusively with slash-commands in mind and as such its data model and declarations are as close to Discord's representation as possible. It also supports automatic registration of commands.

## `serde_steph`

Custom binary serialization format, vaguely inspired by [BARE](https://baremessages.org/). This format is not self-describing and as such deserializing any is disallowed.

The main goal is to be reasonably short and somewhat easy to understand. Slices in the serialized format could be replaced as long as they describe the same structure, even when the size differs.

Created because the `serde_bare` crate was last updated 3 years ago and it's dubious whether it even matches the spec anymore, so there are no real advantages to it anymore. Also that crate allocated memory unconditionally when the deserializer asks for a byte slice. Eh.

No, I did not really read the BARE spec and the output likely isn't compatible. That's not a goal anyways.

## `unity_read`

Allows reading in UnityFS archives, enumerating their files, and objects.

Note that some functionality is not generally applicable, e.g. image decoding and meshes are only implemented for a small subset of the functionality required to work with Azur Lane's data.

Inspired and made by referencing <https://github.com/gameltb/io_unity> and <https://github.com/yuanyan3060/unity-rs> for file formats.

After those libraries gave me trouble I wasn't expecting.

## `utils`

Yup. Essentially a collection of modules I didn't feel like were deserving of their own crate.

Notable modules include:

- `str_as_data`: Provides ways to encode binary data as valid UTF-8 strings and convert those strings back into binary data.
- `text`: Provides helper methods to work with displayed text.
- `fuzzy`: Provides a collection that allows fuzzy text searching.

The actual module documentations may have more info.

## `utils_build`

A collection of helpers for build scripts.
