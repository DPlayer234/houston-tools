use crate::define_unity_class;

define_unity_class! {
    /// Data for Unity's TextAsset class.
    pub class TextAsset = "TextAsset" {
        pub name: String = "m_Name",
        pub script: Vec<u8> = "m_Script",
    }
}
