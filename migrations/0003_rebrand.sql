-- Rebrand the default site identity to 咸鱼图床.
-- Only touches values that are still the shipped defaults, so admins who
-- customized their site name/description are left alone.
UPDATE settings SET value = '咸鱼图床'
  WHERE key = 'site_name' AND value = 'RangPic';
UPDATE settings SET value = '一个自托管的图床服务，随机图只是其中一个功能'
  WHERE key = 'site_description' AND value = '一个自托管的随机图片与图床服务';
