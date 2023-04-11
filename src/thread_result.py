import typing

class _FakeAsync(object):

    def send(self):
        pass
    close = stop = send

    def __call__(self, result):
        "fake out for 'receiver'"

    def __bool__(self):
        return False

    __nonzero__ = __bool__

_FakeAsync = _FakeAsync()

class ThreadResult(object):
    """
    A one-time event for cross-thread communication.

    Uses a hub's "async" watcher capability; it must be constructed and
    destroyed in the thread running the hub (because creating, starting, and
    destroying async watchers isn't guaranteed to be thread safe).
    """

    # Using slots here helps to debug reference cycles/leaks
    __slots__ = ('exc_info', 'async_watcher', 'value',
                 'context', 'hub', 'receiver')

    def __init__(self, receiver, hub):
        self.receiver = receiver
        self.hub = hub
        self.context = None
        self.value = None
        self.exc_info = ()
        self.async_watcher = hub.loop.async_()
        self.async_watcher.start(self._on_async)

    @property
    def exception(self):
        return self.exc_info[1] if self.exc_info else None

    def _on_async(self):
        # Called in the hub thread.

        aw = self.async_watcher
        self.async_watcher = _FakeAsync

        aw.stop()
        aw.close()

        try:
            if self.exc_info:
                self.hub.handle_error(self.context, *self.exc_info)
            self.context = None
            self.async_watcher = _FakeAsync
            self.hub = None

            self.receiver(self)
        finally:
            self.receiver = _FakeAsync
            self.value = None
            if self.exc_info:
                self.exc_info = (self.exc_info[0], self.exc_info[1], None)

    def destroy_in_main_thread(self):
        """
        This must only be called from the thread running the hub.
        """
        self.async_watcher.stop()
        self.async_watcher.close()
        self.async_watcher = _FakeAsync

        self.context = None
        self.hub = None
        self.receiver = _FakeAsync

    def set(self, value):
        self.value = value
        self.async_watcher.send()

    def handle_error(self, context, exc_info):
        self.context = context
        self.exc_info = exc_info
        self.async_watcher.send()

    # link protocol:
    def successful(self):
        return self.exception is None
