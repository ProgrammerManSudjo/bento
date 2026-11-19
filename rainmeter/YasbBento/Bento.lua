local frame = 0
function Initialize() end
function Update() frame = frame + 1 return frame end
-- seconds, to a quarter of a second: the skin updates four times a second
local function T() return frame * 0.25 end

local function now() return os.date('*t') end

function greeting()
  local h = now().hour
  local g = 'GOOD EVENING'
  if h < 5 then g = 'GOOD NIGHT' elseif h < 12 then g = 'GOOD MORNING' elseif h < 18 then g = 'GOOD AFTERNOON' end
  return g .. ', ' .. string.upper(os.getenv('USERNAME') or '')
end

-- the field of dots next to the clock: a slow wave
function wave(i)
  local c, r = i % 16, math.floor(i / 16)
  local t = T()
  local v = (math.sin(c * 0.55 + t * 0.7) * math.cos(r * 0.8 - t * 0.5) + 1) / 2
  return math.floor(22 + v * v * 200)
end

-- the ring of 60 dots: a bright trail ending at the current second
function ring(k)
  local s = tonumber(os.date('%S'))
  local d = (s - k) % 60
  if d < 14 then return math.floor(48 + 207 * (1 - d / 14) ^ 1.4) end
  return 48
end
function ringr(k)
  if tonumber(os.date('%S')) == k then return 4.4 end
  return 2.7
end

-- calendar
local function month_info()
  local n = now()
  local first = os.time{year = n.year, month = n.month, day = 1, hour = 12}
  local wd = (tonumber(os.date('%w', first)) + 6) % 7
  local dim = os.date('*t', os.time{year = n.year, month = n.month + 1, day = 0, hour = 12}).day
  return wd, dim, n.day
end
function monthname() return string.upper(os.date('%B %Y')) end
function day(i)
  local wd, dim = month_info()
  local n = i - wd + 1
  if n >= 1 and n <= dim then return tostring(n) end
  return ''
end
function daycol(i)
  local wd, dim, today = month_info()
  local n = i - wd + 1
  if n < 1 or n > dim then return '0,0,0,0' end
  if n == today then return '0,0,0,255' end
  if n < today then return toward_accent(0.05 + 0.65 * (n / dim)) .. ',225' end
  return '255,255,255,75'
end
-- a ring that swells and fades round today
function pr() return 11.5 + 4.5 * ((T() * 0.7) % 1) end
function pa() return math.floor(190 * (1 - ((T() * 0.7) % 1)) ^ 1.5) end
-- the letter of today's weekday
function wdcol(c)
  local wd, _, today = month_info()
  if c == (wd + today - 1) % 7 then return accent() .. ',255' end
  return '110,110,110,255'
end
function tcol() local wd, _, today = month_info(); return (wd + today - 1) % 7 end
function trow() local wd, _, today = month_info(); return math.floor((wd + today - 1) / 7) end

-- hours of the day
local function grid_distance(i)
  local h = now().hour
  local dc, dr = (i % 6) - (h % 6), math.floor(i / 6) - math.floor(h / 6)
  return math.sqrt(dc * dc + dr * dr)
end
function ha(i)
  local h = now().hour
  if i == h then return 255 end
  local ripple = 0.5 + 0.5 * math.sin(grid_distance(i) * 1.5 - T() * 3.4)
  if i < h then return math.floor(95 + 70 * ripple) end
  return math.floor(32 + 70 * ripple * math.max(0, 1 - grid_distance(i) / 7))
end
function hr(i)
  if i == now().hour then return 5 + 1.8 * (0.5 + 0.5 * math.sin(T() * 3.2)) end
  return 2.3 + 0.9 * math.max(0, math.sin(grid_distance(i) * 1.5 - T() * 3.4)) * math.max(0, 1 - grid_distance(i) / 6)
end
function hourtext() return now().hour .. ' of 24' end

-- uptime
local function up() return SKIN:GetMeasure('mUp'):GetValue() end
function uptext()
  local s = up()
  local d, h, m = math.floor(s / 86400), math.floor(s % 86400 / 3600), math.floor(s % 3600 / 60)
  if d > 0 then return string.format('%dd %dh', d, h) end
  return string.format('%dh %dm', h, m)
end
-- when the machine started
function usince() return 'SINCE ' .. os.date('%H:%M', os.time() - math.floor(up())) end
function ud(i)
  local h = math.floor(up() / 3600)
  if i == h then return 255 elseif i < h then return 190 end
  return 45
end
-- the uptime's bars: a slow wave that never quite repeats, a little taller where the hours have passed
local function bar(i)
  local t = T()
  local v = 0.5 + 0.5 * math.sin(i * 0.62 + t * 1.1)
  local w = 0.5 + 0.5 * math.sin(i * 0.23 - t * 0.7 + 1.3)
  return v * 0.65 + w * 0.35
end
-- a light that sweeps along the bars (and starts over from the left)
local function sweep(i)
  local at = (T() * 7) % 36 - 6
  return math.exp(-((i - at) ^ 2) / 3.5)
end
function ubh(i) return math.floor(6 + 50 * bar(i) + 18 * sweep(i)) end
function udr(i) if i == math.floor(up() / 3600) then return 3.6 end return 2.2 end

-- the accent (Accent.inc, written by YASB Settings' Lucent theme) for the lit dots and today's circle
function accent() return SKIN:GetVariable('Accent', '255,255,255') end
function accent_rgb() return accent() end
function white(a) return '255,255,255,' .. a end
-- a colour that runs from white (f = 0) to the accent (f = 1)
function toward_accent(f)
  f = math.max(0, math.min(1, f))
  local a = {}
  for n in string.gmatch(accent(), '%d+') do a[#a + 1] = tonumber(n) end
  local mix = function(k) return math.floor(255 + (a[k] - 255) * f) end
  return mix(1) .. ',' .. mix(2) .. ',' .. mix(3)
end
-- the clock's dots: white on the left, the accent on the right
function wavec(i)
  return toward_accent(0.15 + 0.85 * ((i % 16) / 15)) .. ',' .. wave(i)
end
-- the seconds: the head is the accent, the trail behind it fades back to white
function ringc(k)
  local s = tonumber(os.date('%S'))
  if s == k then return accent() .. ',255' end
  return toward_accent(math.max(0, 1 - ((s - k) % 60) / 16)) .. ',' .. ring(k)
end
-- the hours and the uptime: the dots that are past run from white to the accent, the current one is the accent
function hc(i)
  if i == now().hour then return accent() .. ',255' end
  return toward_accent(0.1 + 0.9 * (i / 23)) .. ',' .. ha(i)
end
function uc(i)
  local h = math.floor(up() / 3600)
  if i == h then return accent() .. ',255' end
  if i < h then return toward_accent(0.1 + 0.9 * (i / 23)) .. ',' .. math.min(255, 190 + math.floor(65 * sweep(i))) end
  -- the hours still to come: dim, a little brighter as the bar rises
  return toward_accent(0.1 + 0.9 * (i / 23) + 0.3 * sweep(i)) .. ',' .. math.min(255, 45 + math.floor(95 * bar(i)) + math.floor(110 * sweep(i)))
end

-- ---- the calendar: today in a circle, the days around it smaller ----
local function day_at(k)
  local n = now()
  local t = os.time{year = n.year, month = n.month, day = n.day + k, hour = 12}
  return os.date('*t', t), t
end
function bigday() return tostring(now().day) end
function bigwd() return string.upper(os.date('%A')) end
function sideday(k) return tostring((day_at(k)).day) end
-- the circle behind a side day: a little brighter for the days that are past
function sidefill(k)
  local a = math.abs(k)
  local alpha = (k < 0 and 30 or 16) - (a - 1) * 6
  return '255,255,255,' .. math.max(alpha, 8)
end
function sidetext(k)
  local a = math.abs(k)
  local alpha = (k < 0 and 230 or 120) - (a - 1) * 45
  return '255,255,255,' .. alpha
end
-- the glow behind today's circle: slow, soft
function glowgrad()
  local breath = 0.5 + 0.5 * math.sin(T() * 0.8)
  return accent() .. ',' .. math.floor(50 + 40 * breath) .. ' ; 0.0 | ' .. accent() .. ',0 ; 1.0'
end
-- the dots of the month along the bottom: the days that are past in the gradient, today in the accent
function dimn() local _, dim = month_info(); return dim end
function mdot(i)
  local _, dim, today = month_info()
  if i > dim then return '0,0,0,0' end
  if i == today then return accent() .. ',255' end
  if i < today then return toward_accent(0.1 + 0.9 * (i / dim)) .. ',200' end
  return '255,255,255,45'
end
function mdotr(i)
  local _, _, today = month_info()
  if i == today then return 3.6 end
  return 2.2
end

-- ---- the hours: a grid of squares; the hours that are past are filled, this one fills up with its minutes ----
function hcell(i)
  local h = now().hour
  if i < h then return toward_accent(0.1 + 0.9 * (i / 23)) .. ',135' end
  return '0,0,0,0'
end
function hout(i)
  local h = now().hour
  if i == h then return accent() .. ',255' end
  if i < h then return '0,0,0,0' end
  return '255,255,255,40'
end
function hcol() return now().hour % 6 end
function hrow() return math.floor(now().hour / 6) end
function hmin() return math.max(2, math.floor(22 * now().min / 60)) end

-- ---- the uptime: a staircase that grows to the right, one step for every hour since the machine started ----
function ustep(i) return math.floor(8 + 56 * (i / 23) ^ 1.15) end
local function uhours() return math.floor(up() / 3600) end
function uidx() return math.min(23, uhours()) end
-- the bars are mirrored round a middle line, like a soundwave, and they grow to the right: half of a bar's height
local function uhh(i) return 4 + 30 * (i / 23) ^ 1.15 end
function ufill(i)
  local h = uhours()
  if i < h then return toward_accent(0.1 + 0.9 * (i / 23)) .. ',225' end
  -- the empty bars are solid dark greys (nothing shows through them), a little lighter towards the right
  local v = math.floor(24 + 22 * (i / 23))
  if i == h then v = 48 end
  return v .. ',' .. v .. ',' .. v .. ',255'
end
-- the bar that is being filled: out from the middle as the minutes pass (nothing once the machine has been up for a day)
function ufhh()
  local h = uhours()
  if h >= 24 then return 0 end
  return math.max(1, math.floor(uhh(h) * ((up() % 3600) / 3600)))
end
function ucur() return accent() .. ',255' end

-- ---- the hours' pluses: the same slow wave as the clock's dots ----
function plusc(c, r)
  local v = (math.sin(c * 1.1 + T() * 0.7) * math.cos(r * 1.3 - T() * 0.5) + 1) / 2
  return toward_accent(0.15 + 0.85 * (c / 4)) .. ',' .. math.floor(22 + v * v * 215)
end

-- ---- the calendar: a small light drifts round today's circle, slowly ----
function orbx(lag) return 44 * math.cos(T() * 0.35 - lag) end
function orby(lag) return 44 * math.sin(T() * 0.35 - lag) end

-- the line through the days: it fades out at both ends and breathes, a little behind the glow
function calline()
  local breath = 0.5 + 0.5 * math.sin(T() * 0.8 - 1.1)
  return '255,255,255,0 ; 0.0 | 255,255,255,' .. math.floor(34 + 34 * breath) .. ' ; 0.5 | 255,255,255,0 ; 1.0'
end

-- ---- the clock: two soft lights drifting behind the time and the date ----
function clx(i) return 150 + (i == 1 and 46 * math.sin(T() * 0.21) or -52 * math.sin(T() * 0.17 + 1.4)) end
function cly(i) return 84 + (i == 1 and 12 * math.sin(T() * 0.27 + 0.6) or -14 * math.sin(T() * 0.23)) end
function clglow(i)
  local breath = 0.5 + 0.5 * math.sin(T() * 0.5 + i)
  local a = math.floor(26 + 18 * breath)
  if i == 1 then return accent() .. ',' .. a .. ' ; 0.0 | ' .. accent() .. ',0 ; 1.0' end
  return '255,255,255,' .. math.floor(a * 0.5) .. ' ; 0.0 | 255,255,255,0 ; 1.0'
end

-- ---- the clock: a fine grain behind the time, the date and the greeting; each speck swells and fades on its own slow beat ----
-- a speck of the accent: the alpha is scaled by the accent's brightness, so a bright accent does not make bright grain
local function grain_alpha(base)
  local m = 1
  for n in string.gmatch(accent(), '%d+') do m = math.max(m, tonumber(n)) end
  return math.min(255, math.floor(base * math.min(1.4, 140 / m)))
end
function gc(base) return accent() .. ',' .. grain_alpha(base) end
function grain(i, base)
  local wob = 0.5 + 0.5 * math.sin(T() * 0.9 + i * 2.399)
  return accent() .. ',' .. grain_alpha(base * (0.55 + 0.45 * wob))
end
function clbreath()
  local breath = 0.5 + 0.5 * math.sin(T() * 0.4)
  return accent() .. ',' .. math.floor(34 + 18 * breath) .. ' ; 0.0 | ' .. accent() .. ',0 ; 1.0'
end

-- "OCTOBER . 2026" (the dot is a middle dot: Rainmeter reads a script's text in the Windows code page, where it is the single byte 183)
function bigmonth() return string.upper(os.date('%B')) .. ' \183 ' .. os.date('%Y') end
