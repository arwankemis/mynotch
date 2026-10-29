-- Monitor IHSG (^JKSE) untuk MyNotch
-- Data live diambil dari endpoint di config.json (default: Yahoo Finance spark).
-- Sandbox hanya bisa http_get setelah toggle "Net" dinyalakan di Pengaturan -> Mod.
-- Bila izin mati, data belum siap, atau fetch gagal, mod menampilkan data contoh.

local sample_gainers = {
    { ticker = "BREN", name = "Barito Renewables", price = "6.475", change = 12.55 },
    { ticker = "CUAN", name = "Petrindo Jaya Kreasi", price = "3.120", change = 9.86 },
    { ticker = "PANI", name = "Pantai Indah Kapuk Dua", price = "12.300", change = 7.42 },
    { ticker = "AMMN", name = "Amman Mineral Internasional", price = "8.250", change = 6.45 },
    { ticker = "ARTO", name = "Bank Jago", price = "2.780", change = 5.30 }
}

local sample_losers = {
    { ticker = "GOTO", name = "GoTo Gojek Tokopedia", price = "68", change = -6.85 },
    { ticker = "BUKA", name = "Bukalapak", price = "118", change = -5.60 },
    { ticker = "EMTK", name = "Elang Mahkota Teknologi", price = "410", change = -4.65 },
    { ticker = "SIDO", name = "Industri Jamu Sido Muncul", price = "580", change = -3.98 },
    { ticker = "INCO", name = "Vale Indonesia", price = "3.720", change = -3.42 }
}

local yahoo = "https://finance.yahoo.com/quote/%5EJKSE/"
local idx = "https://www.idx.co.id/id/data-pasar/ringkasan-perdagangan/ringkasan-indeks/"

local config = system.config or {}
local endpoints = config.endpoints
if not endpoints and config.endpoint then
    endpoints = { config.endpoint }
end

-- Piccolo hanya menyediakan string.sub/len/reverse/lower/upper dan table.pack/unpack.
-- Jadi format angka dan pengurutan ditulis manual, tanpa string.format/gsub/table.sort.
local function fmt_int(n)
    local s = tostring(math.floor(n + 0.5))
    local out = ""
    local count = 0
    local i = string.len(s)
    while i >= 1 do
        out = string.sub(s, i, i) .. out
        count = count + 1
        if count % 3 == 0 and i > 1 then out = "." .. out end
        i = i - 1
    end
    return out
end

local function fmt_price(n)
    if n >= 1000 then return fmt_int(n) end
    return tostring(math.floor(n + 0.5))
end

local function two_digits(frac)
    if frac < 10 then return "0" .. tostring(frac) end
    return tostring(frac)
end

local function fmt_index(n)
    local scaled = math.floor(n * 100 + 0.5)
    local whole = math.floor(scaled / 100)
    local frac = scaled % 100
    return fmt_int(whole) .. "," .. two_digits(frac)
end

local function fmt_pct(n)
    local scaled = math.floor(math.abs(n) * 100 + 0.5)
    local whole = math.floor(scaled / 100)
    local frac = scaled % 100
    return tostring(whole) .. "," .. two_digits(frac) .. "%"
end

local function signed_pct(n)
    return (n >= 0 and "+" or "-") .. fmt_pct(n)
end

local function strip_symbol(symbol)
    if string.sub(symbol, -3) == ".JK" then
        return string.sub(symbol, 1, string.len(symbol) - 3)
    end
    return symbol
end

local function avg(list)
    if #list == 0 then return 0 end
    local sum = 0
    for _, row in ipairs(list) do sum = sum + row.change end
    return sum / #list
end

local function lane_meta(list)
    local value = avg(list)
    return tostring(#list) .. " emiten · " .. (value >= 0 and "+" or "-") .. fmt_pct(value)
end

local bodies = {}
local pending = false
local blocked = false
local fetch_failed = false
if endpoints and system.http_get and system.json_decode then
    for _, url in ipairs(endpoints) do
        local body, status = system.http_get(url)
        if status == "ok" then
            bodies[#bodies + 1] = body
        elseif status == "pending" then
            pending = true
        elseif status == "blocked" then
            blocked = true
        else
            fetch_failed = true
        end
    end
else
    fetch_failed = true
end

local function sort_desc(rows)
    for i = 2, #rows do
        local item = rows[i]
        local j = i - 1
        while j >= 1 and rows[j].change < item.change do
            rows[j + 1] = rows[j]
            j = j - 1
        end
        rows[j + 1] = item
    end
end

local function parse_bodies()
    local index_meta = nil
    local rows = {}
    for _, body in ipairs(bodies) do
        local data = system.json_decode(body)
        if data and data.spark and data.spark.result then
            for _, item in ipairs(data.spark.result) do
                local response = item.response and item.response[1]
                local meta = response and response.meta
                if meta and meta.regularMarketPrice then
                    if item.symbol == "^JKSE" then
                        index_meta = meta
                    else
                        rows[#rows + 1] = {
                            ticker = strip_symbol(item.symbol),
                            name = meta.shortName or meta.longName or item.symbol,
                            price = fmt_price(meta.regularMarketPrice),
                            change = meta.regularMarketChangePercent or 0
                        }
                    end
                end
            end
        end
    end
    sort_desc(rows)
    local gainers, losers = {}, {}
    for _, row in ipairs(rows) do
        if row.change > 0 and #gainers < 5 then gainers[#gainers + 1] = row end
    end
    for i = #rows, 1, -1 do
        local row = rows[i]
        if row.change < 0 and #losers < 5 then losers[#losers + 1] = row end
    end
    return index_meta, gainers, losers, rows
end

if ui.headline and ui.ranked then
    local gainers, losers = sample_gainers, sample_losers
    local rows = {}
    local index_value, index_delta = "7.648,32", "+32,14 (+0,42%)"
    local badge = "CONTOH DATA"
    local meta = "Data contoh"

    if #bodies > 0 then
        local index_meta, live_gainers, live_losers, live_rows = parse_bodies()
        if index_meta then
            local price = index_meta.regularMarketPrice
            local pct = index_meta.regularMarketChangePercent or 0
            local previous = index_meta.chartPreviousClose or price
            local change = price - previous
            index_value = fmt_index(price)
            index_delta = (change >= 0 and "+" or "-") .. fmt_index(math.abs(change)) .. " (" .. signed_pct(pct) .. ")"
            gainers, losers, rows = live_gainers, live_losers, live_rows
            badge = "LIVE"
            meta = "Yahoo Finance · delayed 15 mnt"
        elseif pending then
            badge = "MEMUAT"
            meta = "Mengambil data Yahoo…"
        else
            meta = "Format data tidak dikenal"
        end
    elseif pending then
        badge = "MEMUAT"
        meta = "Mengambil data Yahoo…"
    elseif blocked then
        meta = "Izin Net mati · data contoh"
    elseif fetch_failed then
        meta = "Live gagal · data contoh"
    end

    -- Lane kiri selalu diisi 5 baris: emiten yang naik dulu, sisanya diisi
    -- pergerakan terbaik berikutnya dan judulnya diganti agar tidak menyesatkan.
    local gainer_label = "Top 5 Gainer"
    local gainer_meta = lane_meta(gainers)
    if #gainers < 5 and #rows > #gainers then
        local positives = #gainers
        for i = positives + 1, #rows do
            if #gainers < 5 then gainers[#gainers + 1] = rows[i] end
        end
        if positives == 0 then
            gainer_label = "Paling Tahan"
            gainer_meta = "Tidak ada emiten naik"
        else
            gainer_label = "Top 5 Teratas"
            gainer_meta = tostring(positives) .. " naik · sisanya terbaik"
        end
    end

    ui.headline("IHSG · IDX Composite", index_value, index_delta, meta, badge)
    ui.ranked("left", gainer_label, gainer_meta, gainers)
    ui.ranked("right", "Top 5 Loser", lane_meta(losers), losers)
else
    -- Fallback untuk MyNotch yang belum punya ui.headline/ui.ranked.
    ui.card("IHSG (^JKSE)", "7.648,32", "+32,14 (+0,42%) · Sesi 2")
    ui.card("Sesi Bursa (WIB)", "13:30–15:49", "Jumat: 14:00–15:49")
    ui.card("Top Gainer", "BREN +12,55%", "CUAN, PANI, AMMN, ARTO")
    ui.card("Top Loser", "GOTO -6,85%", "BUKA, EMTK, SIDO, INCO")

    if ui.button("Chart Yahoo", "ihsg_yahoo") then
        system.open_url(yahoo)
        system.toast("Membuka chart IHSG di Yahoo Finance...")
    end

    if ui.button("IDX Indonesia", "ihsg_idx") then
        system.open_url(idx)
        system.toast("Membuka ringkasan indeks IDX...")
    end

    if ui.button("Salin ringkasan", "ihsg_copy") then
        system.copy("IHSG 7.648,32 (+0,42%) · Gainer: BREN +12,55% · Loser: GOTO -6,85%")
        system.toast("Ringkasan IHSG disalin ke clipboard.")
    end
end
