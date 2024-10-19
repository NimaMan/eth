from typing import Any
from abc import ABC, abstractmethod
from ethblockprocessor.data_models.txn_models import DetailedTransaction


class BaseAlert(ABC):
    @abstractmethod
    def get_alert(self, transaction: DetailedTransaction):
        pass

    @abstractmethod
    def send_alert(self, alert_data: Any):
        pass

    @abstractmethod
    def create_alert(self, transaction: DetailedTransaction):
        pass