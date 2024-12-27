
import websockets
from websockets import WebSocketClientProtocol
import json
from .decorators import retry_async
import asyncio


class ListenerConfiguration():
    def __init__(self, host: str, subscription_request: dict=None):
        self.host = host
        self.subscription_request = subscription_request


class Listener:
    
    def __init__(self, config: ListenerConfiguration):
        self.config: ListenerConfiguration = config
        self.websocket: websockets.ClientProtocol = None
        self.retries = 0

    @retry_async()
    async def send_message(self, message):
        async with websockets.connect(self.config.host, ping_timeout=240) as ws:
            ws: WebSocketClientProtocol
            await ws.send(message)    

    async def subscribe(self, callback):
        async with websockets.connect(self.config.host, ping_timeout=240) as ws:
            ws: WebSocketClientProtocol

            if self.config.subscription_request:
                await ws.send(json.dumps(self.config.subscription_request))
                subscription_response = await ws.recv()
                print(subscription_response)

            while True:
                tmp = await ws.recv()
                message = json.loads(tmp)
                await callback(message)
                self.reset_retries()

    def reset_retries(self):
        self.retries = 0

    async def subscribe_w_retries(self, callback, max_retries: int=4):
        self.reset_retries()
        while self.retries < max_retries:
            try:
                await self.subscribe(callback)
                break  # Exit loop if subscribe is successful
            except Exception as e:
                self.retries += 1
                if self.retries < max_retries:
                    sleep_time = 2 ** self.retries
                    msg = f'Websocket listener failed, retrying in {sleep_time} seconds. Error: {e}'
                    print(msg)
                    self.notify(msg)
                    await asyncio.sleep(sleep_time)
                else:
                    msg = f'Websocket listener died after {max_retries} attempts. Error: {e}'
                    print(msg)
                    self.notify(msg)
                    break # Exit the loop after max retries
                    
    def notify(self, message):
        bot_token = get_bot_token(bot_name='Aladdin Beta')
        channel_id = get_channel_id(channel_name='Aladdin Live Stream')
        send_message(chat_id=channel_id, message=message, bot_token=bot_token)