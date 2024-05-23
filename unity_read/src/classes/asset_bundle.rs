use crate::define_unity_class;

define_unity_class! {
    pub class AssetBundle = "AssetBundle" {
        pub name: String = "m_Name",
        pub preload_table: Vec<AssetPPtr> = "m_PreloadTable",
        pub container: AssetContainer = "m_Container",
    }
}

define_unity_class! {
    pub class AssetContainer = "map" {
        pub array: Vec<AssetEntry> = "Array",
    }
}

define_unity_class! {
    pub class AssetEntry = "pair" {
        pub key: String = "first",
        pub value: AssetInfo = "second",
    }
}

define_unity_class! {
    pub class AssetInfo = "AssetInfo" {
        pub preload_index: i32 = "preloadIndex",
        pub preload_size: i32 = "preloadSize",
        pub asset: AssetPPtr = "asset",
    }
}

define_unity_class! {
    pub class AssetPPtr = "PPtr<Object>" {
        pub file_id: i32 = "m_FileID",
        pub path_id: i64 = "m_PathID",
    }
}
