// Migrate 2.* to 2.15.1 (optional)
// * Only if `starboard` module is enabled.
// * Converts the `pin_messages` field to an array of Int64 values.
db["starboard.messages"].aggregate([
    {
        $match: {
            pin_messages: {
                $type: "string"
            }
        }
    },
    {
        $set: {
            pin_messages: {
                $map: {
                    input: "$pin_messages",
                    in: {
                        $convert: {
                            input: "$$this",
                            to: "long"
                        }
                    }
                }
            }
        }
    },
    {
        $merge: {
            into: "starboard.messages",
            whenNotMatched: "fail"
        }
    }
]);
