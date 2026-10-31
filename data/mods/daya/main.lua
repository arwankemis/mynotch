-- Official optional package. Nil means unavailable, never an invented zero reading.
local snap = system.power_snapshot()
local history = system.power_history()
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
local note = (snap and snap.provider_note) or "Sensor watt belum tersedia."
ui.gauge({ id="observed_energy", value=snap and snap.today_kwh, max=1,
  unit="kWh", label="Teramati hari ini", target_label="Subtotal komponen" })
ui.metrics({ items={
  {label=(snap and snap.cpu_name) or "CPU",value=watts(snap and snap.cpu_watts)},
  {label=(snap and snap.gpu_name) or "GPU",value=watts(snap and snap.gpu_watts)},
  {label="Subtotal sensor",value=watts(snap and snap.active_power_w)},
  {label="Arus / tegangan AC",value="Perlu meter"}
},note=note })
local items={}
if history then
  for i=1,#history do
    local h=history[i]
    if h.provenance == "sensor_component_subtotal" then
      items[#items+1]={id=tostring(i),label=h.name,value=fixed(h.total_kwh or 0, 3) .. " kWh",meta="Sensor CPU/GPU"}
    end
  end
end
ui.history({title="Energi komponen teramati",year=tostring((snap and snap.current_year) or "—"),unit="kWh",items=items})
