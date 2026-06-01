use crate::define_unity_class;

define_unity_class! {
    /// Data for Unity's `TextAsset` class.
    pub class TextAsset = "TextAsset" {
        pub name: String = "m_Name",
        pub script: Vec<u8> = "m_Script",
    }
}

define_unity_class! {
    /// Data for Unity's `TextAsset` class.
    pub class TextAssetRef<'r> = "TextAsset" {
        pub name: &'r [u8] = "m_Name",
        pub script: &'r [u8] = "m_Script",
    }
}
