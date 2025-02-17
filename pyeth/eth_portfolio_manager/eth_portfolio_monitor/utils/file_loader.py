from pathlib import Path


class StaticLoader:
    def __init__(self, static_folder):
        self.static_folder = static_folder
        self.cache = {}
        self._load_static_files()
    
    def _read_file(self, file_path):
        with open(Path(self.static_folder, file_path), 'r') as file:
            return file.read()
    
    def _load_static_files(self):
        self.cache = {
            'styles_css': self._read_file('css/styles.css'),
            'chain_stats_js': self._read_file('js/chain_stats.js'),
            'bs_init_js': self._read_file('assets/js/bs-init.js'),
            'theme_js': self._read_file('assets/js/theme.js'),
            'styles_bootstrap_css': self._read_file('assets/bootstrap/css/bootstrap.min.css')
        }
    
    def get_static_files(self):
        return self.cache