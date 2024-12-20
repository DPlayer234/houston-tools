-- Set up loader to support paths relative to working dir
package.path = AZUR_LANE_DATA_PATH .. "/?.lua"

-- Set up data loading like AL does. Mostly.
pg = {}
ys = {}
cs = {}

HXSet = {}

function HXSet.hxLan(text)
    return string.gsub(text or "", "{namecode:(%d+).-}", function (match)
        local name_data = pg.name_code[tonumber(match)];
        return name_data and name_data.name
    end)
end

local function translate_equip_data_code(text)
    return string.gsub(text or "", "<%[(.-)%]>", function (match)
        local name_data = pg.equip_data_code[match]
        return name_data and name_data.text
    end)
end

local function lazy_load(mode, allow_name_code)
    return function(args, index)
        local name = args.__name;

        if mode == 1 and cs[name][index] then
            -- The sharecfgdata files are separate from the main game script.
            -- In this case however, we just have them decompiled already, so just run them.
            -- LuaHelper.SetConfVal(name, cs[name][index][1], cs[name][index][2])
            require("sharecfgdata." .. name)
        end

        if mode == 2 and cs[name].indexs[index] then
            local subName = cs[name].subList[cs[name].indexs[index]]
            if pg.base[subName] == nil then
                require("sharecfg." .. cs[name].subFolderName .. "." .. subName)
            end

            name = subName
        end

        local data = pg.base[name][index]
        if not data then
            return nil
        end

        local real = {}
        for k, v in pairs(data) do
            if type(v) == "string" then
                real[k] = translate_equip_data_code(v);

                if allow_name_code then
                    real[k] = HXSet.hxLan(real[k])
                end
            end
        end

        local base_id = rawget(data, "base")
        if base_id ~= nil then
            args[index] = setmetatable(real, {
                __index = function (self, key)
                    local raw = data[key]
                    if raw == nil then
                        return args[base_id][key]
                    else
                        return raw
                    end
                end
            })
        else
            args[index] = setmetatable(real, {
                __index = data
            })
        end

        return args[index]
    end
end

-- These tables are used as metatable by the resource lookup tables.
confSP = { -- Use sublist files
    __index = lazy_load(2, true)
}
confMT = { -- Load sharecfgdata file first
    __index = lazy_load(1, true)
}
confHX = { -- Immediately accessible
    __index = lazy_load(0, true)
}

-- Accessed by some loaded scripts and dummied out
ys.Battle = {
    BattleDataFunction = {
        ConvertBuffTemplate = function() end,
        ConvertSkillTemplate = function() end
    }
}

-- cursed fix for some scripts
uv0 = setmetatable({}, {
    __index = function() return {} end
});

require("localconfig")
require("const")
require("config")
require("buffcfg")
require("skillcfg")

-- Enable lazy-loading the resource tables themselves.
setmetatable(pg, {
    __index = function (self, index)
        if ShareCfg["ShareCfg." .. index] then
            require("sharecfg." .. index)
            return rawget(self, index)
        end
    end
})

-- Used by our code to load a buff/skill.
function require_buff(id)
    if pg.buffCfg_tag["buff_" .. id] then
        return require("gamecfg.buff.buff_" .. id)
    end
end

function require_skill(id)
    if pg.skillCfg_tag["skill_" .. id] then
        return require("gamecfg.skill.skill_" .. id)
    end
end

-- Helper for augment parsing
function get_augment_ship_types(kind)
    local sp = pg.spweapon_type[kind]
    if sp then
        return sp.ship_type
    end
end
