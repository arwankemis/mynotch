-- Contoh mod skrip Lua untuk MyNotch
ui.card("Status Modul", "Aktif", "Berjalan di sandbox piccolo")
ui.card("Waktu Sesi", "25 Menit", "Rekomendasi fokus berikutnya")

if ui.button("Kirim Sapaan", "btn_greet") then
    system.toast("Halo! Skrip Lua MyNotch bekerja normal.")
end

if ui.button("Buka Repositori", "btn_repo") then
    system.open_url("https://github.com/arwankemis/mynotch")
    system.toast("Membuka repositori GitHub...")
end

if ui.button("Salin Teks Contoh", "btn_copy") then
    system.copy("Teks ini dihasilkan oleh mod Lua MyNotch.")
    system.toast("Teks disalin ke papan klip.")
end
