-- Seed built-in themes if none exist
INSERT OR IGNORE INTO themes (name, data, is_active) VALUES
    ('dark', '{"name":"dark","bg":"#1e1e2e","fg":"#cdd6f4","accent":"#89b4fa","selection":"#313244","border":"#585b70","error":"#f38ba8","success":"#a6e3a1","warning":"#f9e2af","muted":"#6c7086"}', 1),
    ('light', '{"name":"light","bg":"#eff1f5","fg":"#4c4f69","accent":"#1e66f5","selection":"#ccd0da","border":"#9ca0b0","error":"#d20f39","success":"#40a02b","warning":"#df8e1d","muted":"#8c8fa1"}', 0),
    ('solarized', '{"name":"solarized","bg":"#002b36","fg":"#839496","accent":"#268bd2","selection":"#073642","border":"#586e75","error":"#dc322f","success":"#859900","warning":"#b58900","muted":"#657b83"}', 0),
    ('gruvbox', '{"name":"gruvbox","bg":"#282828","fg":"#ebdbb2","accent":"#83a598","selection":"#3c3836","border":"#665c54","error":"#fb4934","success":"#b8bb26","warning":"#fabd2f","muted":"#928374"}', 0),
    ('nord', '{"name":"nord","bg":"#2e3440","fg":"#d8dee9","accent":"#88c0d0","selection":"#3b4252","border":"#4c566a","error":"#bf616a","success":"#a3be8c","warning":"#ebcb8b","muted":"#616e88"}', 0),
    ('last-horizon', '{"name":"last-horizon","bg":"#0c0b0c","fg":"#e2dddc","accent":"#b59790","selection":"#3a2f30","border":"#3a2f30","error":"#c4d8e2","success":"#87a9b0","warning":"#c38b7b","muted":"#9b7369"}', 0);
