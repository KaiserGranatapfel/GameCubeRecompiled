-- Game initialization script
-- Loaded at game startup to register Lua-defined menus and handlers

print("[game] Lua game scripts loading...")

-- Load menu definitions
local menus = require("lua.game.menus")

print("[game] Lua game scripts loaded successfully")
print("[game] Registered " .. #menus .. " menu screens")
