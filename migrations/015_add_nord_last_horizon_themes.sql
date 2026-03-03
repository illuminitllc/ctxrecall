-- Add nord and last-horizon built-in themes
INSERT OR IGNORE INTO themes (name, data, is_active) VALUES
    ('nord', '{"name":"nord","bg":"#2e3440","fg":"#d8dee9","accent":"#88c0d0","selection":"#3b4252","border":"#4c566a","error":"#bf616a","success":"#a3be8c","warning":"#ebcb8b","muted":"#616e88"}', 0),
    ('last-horizon', '{"name":"last-horizon","bg":"#0c0b0c","fg":"#e2dddc","accent":"#b59790","selection":"#3a2f30","border":"#3a2f30","error":"#c4d8e2","success":"#87a9b0","warning":"#c38b7b","muted":"#9b7369"}', 0);
