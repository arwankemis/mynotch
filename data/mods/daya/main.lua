-- Daya: Energy & Power Monitor Mod
local snap = system.power_snapshot and system.power_snapshot()
local history = system.power_history and system.power_history(2026)

local active_w = (snap and snap.active_power_w) or 0.0
local today_kwh = (snap and snap.today_kwh) or 0.0

-- 1. Gauge widget
ui.gauge({
    id = "daya_gauge",
    value = today_kwh,
    max = 10.0,
    unit = "kWh",
    label = "kWh · hari ini",
    target_label = "Target: 10,0 kWh"
})

-- 2. Metrics 2x2 grid
local watt_str = tostring(active_w) .. " W"
local cost_str = "Rp 0"
if snap and snap.monthly_cost_idr then
    cost_str = "Rp " .. tostring(snap.monthly_cost_idr) .. "/bln"
end
local current_str = "--"
if snap and snap.current_a then
    current_str = tostring(snap.current_a) .. " A"
end
local volt_str = "--"
if snap and snap.voltage_v then
    volt_str = tostring(snap.voltage_v) .. " V"
end

local source_note = "Estimasi berbasis beban CPU/GPU"
if snap and snap.source_kind == "measured" then
    source_note = "Pengukuran fisik (Power Meter)"
end

ui.metrics({
    items = {
        { label = "Daya", value = watt_str },
        { label = "Biaya", value = cost_str },
        { label = "Arus", value = current_str },
        { label = "Tegangan", value = volt_str }
    },
    note = source_note
})

-- 3. Monthly History
local hist_items = {}
if history and #history > 0 then
    for i = 1, #history do
        local h = history[i]
        local label = h.name or ("Bulan " .. tostring(h.month or i))
        local val = tostring(h.total_kwh or 0) .. " kWh"
        local meta = h.provenance or "arsip"
        hist_items[#hist_items + 1] = {
            id = tostring(h.month or i),
            label = label,
            value = val,
            meta = meta
        }
    end
else
    hist_items[1] = {
        id = "current",
        label = "Bulan ini",
        value = tostring(today_kwh) .. " kWh",
        meta = "berjalan"
    }
end

ui.history({
    title = "Riwayat bulanan",
    year = "2026",
    unit = "kWh",
    items = hist_items
})
