-- Official optional package. Nil means unavailable, never an invented zero reading.
local snap = system.power_snapshot()
local history = system.power_history()
local lang = (snap and snap.language) or (system and system.language and system.language()) or "id"
local is_en = (lang == "en")

local function fixed(value, digits)
  local scale = 10 ^ digits
  local n = math.floor(value * scale + 0.5)
  local whole = math.floor(n / scale)
  local tail = tostring(n - whole * scale)
  while #tail < digits do tail = "0" .. tail end
  return tostring(whole) .. "." .. tail
end
local function watts(value)
  if value == nil then return "—" end
  return fixed(value, 1) .. " W"
end
local function gigabytes(used, total)
  if used == nil then return "—" end
  if total ~= nil then
    return fixed(used, 1) .. " / " .. fixed(total, 0) .. " GB"
  end
  return fixed(used, 1) .. " GB"
end
local function storage_gb(used, total)
  if used == nil then return "—" end
  if total ~= nil then
    return fixed(used, 0) .. " / " .. fixed(total, 0) .. " GB"
  end
  return fixed(used, 0) .. " GB"
end

local raw_gpu_w = snap and snap.gpu_watts
if raw_gpu_w and raw_gpu_w > 175.0 then
  raw_gpu_w = nil
end
local cpu_val = watts(snap and snap.cpu_watts)
local gpu_val = watts(raw_gpu_w)
local ram_val = gigabytes(snap and snap.ram_used_gb, snap and snap.ram_total_gb)
local storage_val = storage_gb(snap and snap.storage_used_gb, snap and snap.storage_total_gb)

local cpu_pct = snap and snap.cpu_usage
local gpu_pct = snap and snap.gpu_usage
local ram_pct = snap and snap.ram_usage
local storage_pct = snap and snap.storage_usage

local note = (snap and snap.provider_note) or (is_en and "Watt sensor not yet available." or "Sensor watt belum tersedia.")
ui.gauge({
  id="observed_energy",
  value=snap and snap.today_kwh,
  max=1,
  unit="kWh",
  label=is_en and "Observed today" or "Teramati hari ini",
  target_label=is_en and "Component subtotal" or "Subtotal komponen"
})

ui.metrics({ items={
  {
    label=(snap and snap.cpu_name) or "CPU",
    value=cpu_val,
    percent=cpu_pct,
    progress=cpu_pct and (cpu_pct / 100) or nil
  },
  {
    label=(snap and snap.gpu_name) or "GPU",
    value=gpu_val,
    percent=gpu_pct,
    progress=gpu_pct and (gpu_pct / 100) or nil
  },
  {
    label="RAM",
    value=ram_val,
    percent=ram_pct,
    progress=ram_pct and (ram_pct / 100) or nil
  },
  {
    label=is_en and "Storage" or "Penyimpanan",
    value=storage_val,
    percent=storage_pct,
    progress=storage_pct and (storage_pct / 100) or nil,
    has_dropdown=true
  }
}, note=note })

local items={}
if history then
  for i=1,#history do
    local h=history[i]
    if h.provenance == "sensor_component_subtotal" then
      items[#items+1]={
        id=tostring(i),
        label=h.name,
        value=fixed(h.total_kwh or 0, 3) .. " kWh",
        meta=is_en and "CPU/GPU sensor" or "Sensor CPU/GPU"
      }
    end
  end
end
ui.history({
  title=is_en and "Observed component energy" or "Energi komponen teramati",
  year=tostring((snap and snap.current_year) or "—"),
  unit="kWh",
  items=items
})
