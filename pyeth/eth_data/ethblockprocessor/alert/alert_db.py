import lmdb
import json
import os
from typing import List, Dict, Any
from dataclasses import asdict


class AlertDB:
    def __init__(self, path=None, map_size=1*1024*1024*1024):
        if path is None:
            path = os.path.join(os.environ.get("ETH_ADDRESS_DIR", ""), "alerts", "alert_store.lmdb")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        self.env = lmdb.open(path, map_size=map_size)

    def add_alerts(self, alerts: List[Dict[str, Any]]):
        with self.env.begin(write=True) as alert_db_txn:
            for alert in alerts:
                alert_data = asdict(alert)
                alert_type = alert_data['alert_type']
                key = alert_type.encode('utf-8')
                existing_data = alert_db_txn.get(key)
            
                if existing_data:
                    alerts = json.loads(existing_data.decode('utf-8'))
                    alerts.append(alert_data)
                else:
                    alerts = [alert_data]
            
                alert_db_txn.put(key, json.dumps(alerts).encode('utf-8'))

    def get_alerts(self, alert_type: str = None) -> List[Dict[str, Any]]:
        with self.env.begin() as txn:
            if alert_type:
                key = alert_type.encode('utf-8')
                data = txn.get(key)
                return json.loads(data.decode('utf-8')) if data else []
            else:
                alerts = []
                for key, value in txn.cursor():
                    alerts.extend(json.loads(value.decode('utf-8')))
                return alerts

    def get_alert_by_transaction(self, transaction_hash: str) -> List[Dict[str, Any]]:
        alerts = []
        with self.env.begin() as txn:
            for _, value in txn.cursor():
                alert_list = json.loads(value.decode('utf-8'))
                alerts.extend([alert for alert in alert_list if alert['transaction_hash'] == transaction_hash])
        return alerts

    def reset_database(self):
        self.close()
        db_path = self.env.path()
        if os.path.exists(db_path):
            os.remove(db_path)
        if os.path.exists(db_path + "-lock"):
            os.remove(db_path + "-lock")
        self.env = lmdb.open(db_path, map_size=self.env.info().map_size)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback):
        self.close()

    def close(self):
        if self.env:
            self.env.close()
            self.env = None
