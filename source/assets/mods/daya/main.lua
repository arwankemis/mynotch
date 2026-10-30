-- Official optional package. Nil means unavailable, never an invented zero reading.
local snap = system.power_snapshot()
local history = system.power_history()
local function watts(value)
  if value == nil then return "—" end
  return tostring(value) .. " W"
end
local note = (snap and snap.provider_note) or "Sensor watt belum tersedia."
if snap and snap.active_power_w ~= nil then
  note = "Subtotal sensor CPU/GPU; bukan daya total PC atau stopkontak."
end
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
    items[#items+1]={id=tostring(i),label=h.name,value=tostring(h.total_kwh).." kWh",meta=h.provenance}
  end
end
ui.history({title="Energi komponen teramati",year=tostring((snap and snap.current_year) or "—"),unit="kWh",items=items})
