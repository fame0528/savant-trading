[0:00]
What you're looking at here is the

[0:01]
results table after Claude code

[0:04]
controlled my computer overnight. And

[0:06]
this was over 8 hours with one prompt

[0:09]
non-stop. So, it built the momentum

[0:11]
squeeze indicator. And this is after it

[0:14]
tried several different strategies. Here

[0:16]
was the first one. You can see the

[0:18]
drawdown is 73%, and this failed its

[0:21]
test. I told it it has to have a high

[0:23]
enough sharp ratio, and it has to have a

[0:24]
low enough drawdown. Now, the net profit

[0:26]
we don't care so much about because we

[0:28]
want to have really good sharp and

[0:31]
drawdown ratios. People who chase net

[0:33]
profit don't end up doing so well in

[0:35]
real trading because it's these other

[0:38]
metrics that matter the most. So, you

[0:40]
can see this version has slightly lower

[0:42]
drawdown. This was the second version it

[0:44]
built overnight. After testing it, this

[0:46]
is the MFI regime indicator that it

[0:49]
built. And these metrics here in the

[0:51]
table did not work. And we can have This

[0:54]
is the in-sample data. We also have

[0:56]
out-of-sample, so candlesticks we've

[0:57]
withheld from the system. And it still

[1:01]
performed quite well with net profit,

[1:03]
but that max drawdown again is just far

[1:06]
too high. So, the next version that it

[1:08]
made slightly better. This is a little

[1:11]
bit better. There's more trades. We have

[1:12]
a 40% win rate. Sharp ratio came up to

[1:14]
0.3 in the in-sample testing. And in the

[1:17]
out-of-sample, meaning candlesticks that

[1:19]
we only show to it after it's configured

[1:21]
all of the system to see how it responds

[1:24]
to new data. Not bad. I mean, the max

[1:26]
drawdown came out to 28% here. Sharp

[1:28]
ratio's 4 0.4. You know, we want that to

[1:31]
come up. And win rate is 36%. So, not

[1:33]
bad. Like it's it's still profitable in

[1:35]
the out-of-sample, but that drawdown

[1:37]
still a little bit too high. The next

[1:39]
version was 53% net drawdown in the

[1:42]
in-sample. So, still it's progressively

[1:45]
getting better, as you can see. And the

[1:47]
win rate's still teetering around that

[1:49]
37%, and the out-of-sample testing,

[1:52]
let's just show you the metrics for this

[1:54]
table out-of-sample. You know, 30%

[1:56]
drawdown is coming down progressively.

[1:59]
Sharp ratio is slightly going up to

[2:01]
0.42. And of course, we want to get that

[2:04]
better. And so here is the second last

[2:06]
version that it made. Now the net profit

[2:08]
is 566. Pretty pretty okay. Like it's

[2:11]
starting to look a little bit better

[2:13]
now. There's a lot more trades, so a lot

[2:15]
more sample to base our decisions off.

[2:17]
Sortino came up to above two. Sharp

[2:19]
ratio is 0.4. Win rate 45% and the max

[2:23]
drawdown is 33%. Now keep in mind we

[2:25]
started way way way back back here. 73%

[2:28]
net drawdown with a Kelt it's a Keltner

[2:31]
EMA system that it tried very first. And

[2:34]
all of these systems that it built, it

[2:36]
tested them and refined them overnight.

[2:39]
So I didn't have to prompt it at all for

[2:42]
any of this. It just built it and it

[2:44]
used my trading engine that I have

[2:45]
locally, which I will teach you guys how

[2:47]
to set up in a video later this week.

[2:50]
And I'm using Vector BT and also a lot

[2:54]
of customization with it. So Vector BT

[2:57]
can just flash all the candlesticks at

[2:58]
once locally. So you download it from

[3:00]
the exchange. You process them, so you

[3:02]
slice up the data so that you have in

[3:04]
sample and out of sample. So you just

[3:06]
train your parameters to see if they

[3:07]
pass the metrics to see if you're

[3:09]
optimizing it, right? So you're testing

[3:11]
all the parameter combinations and then

[3:13]
you're selecting the ones that pass. And

[3:15]
then you have to validate them by doing,

[3:17]
you know, bootstrapping uh which is just

[3:19]
randomizing the candlesticks as I've

[3:22]
shown in previous videos. But basically

[3:24]
the bootstrapping is the hardest thing

[3:26]
to do cuz when the candlesticks are

[3:28]
randomized, it might do worse. Look at

[3:29]
this. And so, you know, that Sortino

[3:32]
ratio is far lower. The sharp ratio is

[3:34]
not negative. You know, it's still

[3:35]
positive, but you want to make sure that

[3:37]
it survives in these simulations. Like I

[3:39]
can show you other simulations here.

[3:41]
This is another randomized candlesticks.

[3:43]
Uh and it's still it's it's still

[3:45]
holding its ground, but not quite

[3:48]
perfectly as you can see here. So from

[3:50]
the this was the momentum squeeze, but

[3:53]
we do have uh our final version here

[3:55]
that it built. And the max drawdown in

[3:57]
the out of sample is only 19%. Sharp

[4:00]
ratio came way up to 0.66 in this

[4:03]
version. Total trades 409. So, we have a

[4:05]
large enough sample size. And in the in

[4:07]
sample, let's see how it performed in

[4:10]
sample versus out of sample. So, sharp

[4:12]
ratio is actually worse in the in

[4:14]
sample, and the win rate's 45%, but the

[4:17]
win rate is very, very similar in the

[4:18]
out of sample. So, 45% win rate, 44%.

[4:22]
Ba- 44.7. So, pretty much 45% win rate

[4:25]
in the in sample versus out of sample.

[4:27]
You want to see the least amount of

[4:29]
divergence that are with these metrics.

[4:30]
So, you want the sharp ratio, the

[4:32]
drawdown to be relatively similar. In

[4:34]
the in sample, we do have a little bit

[4:35]
more drawdown at I think uh around 30%,

[4:39]
and in the out of sample data, around

[4:40]
20%. So, this system utilizes a momentum

[4:44]
squeeze uh and then it has some trade

[4:46]
filters. And it also I don't think it

[4:49]
picked up any exits. So, we don't have

[4:51]
even exit exit rules configured for

[4:53]
this. But, you know, if I let this

[4:55]
system run for a couple more evenings

[4:57]
overnight, 8 hours, 16 hours, 24 hours,

[5:00]
or even for a full week overnight, it

[5:02]
would take this system and then just

[5:03]
keep refining it. So, it would try to

[5:05]
build new trading rules for it to add to

[5:07]
it to see if it can be improved further.

[5:10]
And that is the beauty of using Claude

[5:12]
code. And that And that is the beauty of

[5:14]
using Claude code overnight. So, it's

[5:17]
really easy to set up Claude code. You

[5:19]
know, you just download Visual Studio

[5:22]
Code or any other programming tool.

[5:24]
There's Cursor. I personally like Visual

[5:26]
Studio Code, and I install Claude Code

[5:28]
on it. And I also use Droid as well, cuz

[5:30]
it injects better system prompts into

[5:32]
it.

[5:33]
And just to show you, Droid is currently

[5:34]
topping the Terminal Bench leaderboards

[5:36]
here, and they're using a GPT-5 CodeX as

[5:40]
the number one. It's able to score

[5:43]
extremely high on the Terminal Bench

[5:45]
leaderboards. Now, I'm using Droid with

[5:47]
ClaudeOpus 4.6, so it's a little bit

[5:49]
lower down at only 70% versus 77,

[5:52]
meaning it only solves 70% of the coding

[5:55]
problems given to it. But, the kind of

[5:57]
coding problems it's able to solve are

[5:58]
kind of analogous to the work that I'm

[6:01]
doing. So, it's good enough for me. And

[6:02]
also, I really like the Claude

[6:04]
ecosystem. And when it runs the system

[6:07]
overnight, it cranks up my computer, but

[6:09]
ClaudeOpus just sits there in idle. So,

[6:11]
it's just waiting and sleeping. You can

[6:13]
see the start sleep command here. So, it

[6:15]
actually doesn't use that many tokens

[6:17]
while it's running overnight for the

[6:19]
whole 8 hours cuz most of that is it's

[6:21]
just sleeping and waiting for the

[6:23]
results to to come in. And it will

[6:25]
extract these results from the

[6:27]
dashboards that I have here. It extracts

[6:29]
these, and it can look at all of the

[6:31]
metrics from this entire system here

[6:34]
that I have. Now, here is the the the

[6:36]
Monte Carlo simulations. And you want to

[6:39]
see less variation in these, more

[6:40]
stability in randomize.

[6:43]
And I've programmed into my system

[6:44]
prompts kind of what I want it to do.

[6:47]
So, I want it to start by building a

[6:49]
backbone, a really solid foundation for

[6:51]
the strategy, and then it will work from

[6:54]
there. So, for example, if the backbone,

[6:56]
this was the momentum squeeze that

[6:58]
performed really well in testing. This

[7:00]
was just the base, this is the base

[7:01]
version here, for example. So, this is

[7:03]
the bootstrap testing with no additional

[7:06]
filters. You can see it's a little bit

[7:07]
choppy there. That's not desirable. But,

[7:10]
compare this side to side with the final

[7:12]
version here with the trade filters. And

[7:14]
let's just look here at how it performs

[7:17]
analogously. So, it's it's still not

[7:20]
doing the best because we're jumbling up

[7:22]
the candlesticks, but it's a little bit

[7:25]
better than this severe chop here. And

[7:27]
again, these simulations aren't

[7:28]
analogous. These weren't an entirely the

[7:30]
same data set it ran on cuz it's random.

[7:33]
But, the best performance that it was

[7:34]
able to do, it still counted this as,

[7:37]
you know, 40% win rate in these

[7:38]
simulations. And so, it it it passed it.

[7:41]
But, if I kept going with this, you

[7:44]
know, we have a couple chop market

[7:45]
filters, so it won't trade you know when

[7:47]
the market Let's see the market is

[7:49]
exceptionally difficult and like here

[7:51]
for example where there's a lot of

[7:52]
liquidity sweeps coming in, it it it it

[7:55]
should avoid these trading periods and

[7:57]
only kind of activate and awake from its

[8:00]
sleep once we have really volatile

[8:02]
periods like in in these few days here

[8:04]
cuz the market will typically trend

[8:07]
horizontally or really challenge like

[8:10]
really challenging periods except for a

[8:12]
couple days like these really big moves

[8:14]
when a lot of the volatility turns up.

[8:16]
So, it's pretty easy to get your bot to

[8:18]
go to sleep for the duration of these

[8:20]
periods here like you do not want it to

[8:22]
trade in these zones. You you really

[8:24]
want it to activate once there's more

[8:25]
volume for example. If there's a high

[8:27]
enough volume on chain, then what you

[8:29]
can use on balance volume OBV or you can

[8:32]
use a bunch of other indicators to

[8:34]
indicate the strength of a trend if it's

[8:36]
really really clear which direction that

[8:38]
your bot should be trading in it will

[8:40]
only trade in that direction just so

[8:41]
that you you know you're trying to give

[8:43]
yourself an edge over the market. And

[8:45]
again, you know, a lot of the a lot of

[8:47]
the market functions already off trading

[8:49]
bots. You can you can look at for

[8:51]
example Bitcoin and Ethereum or all the

[8:54]
other altcoins, they all corresponds

[8:56]
together and it's because a lot of

[8:58]
industries are just using trading bots

[9:00]
to extract value from the markets. And

[9:02]
if your trading bot can get us even a

[9:04]
small portion of that pie, if it's tuned

[9:06]
properly, then yeah, you absolutely can

[9:10]
you can you might not get as much money

[9:13]
as these really really big institutional

[9:15]
bots with you know and a lot of them can

[9:17]
like trigger liquidity sweeps and things

[9:19]
like that. But, as long as you're

[9:21]
getting a little bit of it, you have an

[9:22]
edge. 51% win rate is still over a long

[9:25]
period of time like if you know gambling

[9:27]
and blackjack, you know that somebody

[9:29]
who wins in blackjack 51% of the time

[9:32]
eventually will be profitable. And so,

[9:34]
what are my main bots built off? Well,

[9:36]
it's the it's the Wolf Pack indicator

[9:38]
that I have here. You can see very clear

[9:39]
signals here. Really strong bearish

[9:41]
diamonds here coming in. And so, these

[9:44]
signals are fairly obvious for my

[9:46]
trading bot. For example, we have a teal

[9:48]
RSI here showing that the RSI is very

[9:51]
oversold here. So, I you know,

[9:53]
personally on Ethereum we're expecting a

[9:55]
bounce here potentially coming in. And

[9:57]
so, I use this as one of the backbones

[9:59]
for my trading bots that has performed

[10:01]
very well for myself in real time using

[10:03]
real money on a real trading account.

[10:05]
So, what you're seeing here are the back

[10:07]
This is a backtesting system. And this

[10:09]
is where I spend most of my time because

[10:11]
developing a trading system, uh you want

[10:13]
to have just a slight edge over the

[10:16]
market. And that edge might not last

[10:18]
forever, right? It you know, obviously

[10:20]
every trading system doesn't function in

[10:22]
every market. That's why I'm adding all

[10:24]
these trading filters on. You can see

[10:26]
the trading filters on my Wolf Pack bot.

[10:28]
There's a lot of these periods where we

[10:30]
have the the yellow coming in showing a

[10:33]
little bit of flat markets that my bot

[10:35]
would, you know, try not to trade in

[10:37]
these markets. But, it would want to

[10:38]
trade when there are more volatilities

[10:41]
like this period. It would It would want

[10:43]
to trade here, for example. So, you want

[10:45]
to make it clear for your bot exactly

[10:47]
when to trade and when And it's actually

[10:49]
more important when not to trade for

[10:51]
your trading engine. Your trading engine

[10:53]
just might be tuned for, you know, a

[10:55]
more

[10:56]
maybe when the market is completely

[10:58]
capitulating and people are getting

[11:01]
liquidated, that's a really good time to

[11:03]
turn your bot on. For example, it's

[11:04]
really clear what's happening. And then

[11:06]
in the in between, when it's unclear if

[11:07]
it's going up or down, the bot could

[11:09]
just sleep and wait for that

[11:10]
opportunity. Whereas for you as a human,

[11:12]
you have bias, you bring those biases

[11:14]
into it, and you're, you know, you might

[11:16]
want the the market to go up. And so,

[11:18]
you have this implicit bias you're

[11:20]
taking into your trading. But, a trading

[11:21]
bot is able to take in infinite amounts

[11:24]
of information that as a human, it's

[11:26]
very hard to process that that

[11:28]
information live in real time 24/7 and

[11:32]
and trying to eliminate your biases that

[11:33]
you have as well. Whereas a trading bot

[11:35]
just looks at the math. It's looking at

[11:37]
and it's processing more math than you

[11:38]
can with a human brain. And then it's

[11:40]
able to execute on that perfectly. So,

[11:42]
what the reason we're using that I

[11:44]
personally love using trading bots and

[11:46]
algorithmic trading, especially using

[11:48]
Cloud Overnight, is just it's it's

[11:50]
really becomes effortless. Now, setting

[11:52]
a system up like this is quite difficult

[11:55]
as it can be very finicky, but there are

[11:58]
systems like Vector BT, like I said, and

[12:00]
you know, I'll just use a quick example.

[12:02]
Setting up Vector BT can be as simple as

[12:04]
just asking CloudOpus, "Hey, we need to

[12:07]
download chart data from, let's just

[12:09]
say, your favorite stock broker. We need

[12:11]
to download that stock data or

[12:14]
crypto exchange data. Download that,

[12:15]
splice that into training data and

[12:17]
testing data. We got to run backtests on

[12:19]
it using Vector BT. So, we need a

[12:21]
trading engine for that, but Vector BT

[12:23]
already has one for you." And it's it's

[12:25]
really that simple to set it up locally.

[12:27]
And then from there, it's just

[12:28]
displaying it You know, you don't have

[12:30]
to have a pretty table and pretty

[12:32]
display like I have here, but you can if

[12:34]
you want it if you want to spend the

[12:35]
time to do so. Otherwise, you could just

[12:37]
have kind of the numbers on screen, so

[12:38]
and Cloud knows what those numbers mean

[12:40]
either way. So, from the results, it can

[12:42]
know whether what counts as a pass, what

[12:44]
counts as a fail, for example. And it's

[12:46]
really that easy to set up locally. And

[12:49]
a lot of these run on a CPU, so if you

[12:51]
have like an okay CPU, it doesn't run on

[12:54]
GPU. It It will actually tend to perform

[12:58]
overnight for you. Now, you know, you

[13:00]
might hear your your computer fan

[13:01]
blasting on maximum

[13:04]
maximum fan, but, you know, it's doing

[13:07]
This is able to do thousands of

[13:09]
backtests per second. And we're talking

[13:12]
deep backtests. Now, on TradingView, if

[13:13]
you're to do a deep backtest, it might

[13:15]
take, you know, if you if we open up our

[13:17]
strategy here and we want to do a deep

[13:18]
backtest, let's just say cut the entire

[13:21]
history, it's updating the report,

[13:22]
updating, might take a couple seconds

[13:24]
just to do one. And if I'm on a lower

[13:26]
time frame, if I come down to the

[13:27]
30-minute chart, it's you know, if I

[13:29]
click entire history here, it's going to

[13:31]
take 10, 20 seconds maybe just to do

[13:34]
one. You know, it's still updating here

[13:36]
since switching over to the 30-minute

[13:38]
chart. And you know, that's the

[13:40]
difference between calling, you know,

[13:42]
trading these servers, requesting that

[13:44]
them to get the trading data for you,

[13:46]
running all of your math for your

[13:48]
trading indicator through their system,

[13:50]
and then getting the results. You know,

[13:52]
when you have all the data downloaded

[13:54]
locally, it's really quick. All right,

[13:56]
it's just using your computer locally.

[13:57]
There's no calls to an exchange. You

[13:59]
know, your your network is not based on

[14:01]
network latency, and it's not based on

[14:03]
what little uh processing power

[14:05]
TradingView gives you. So, this is how

[14:07]
we're able to do all these tests, and

[14:09]
then they plug in one to one to

[14:11]
TradingView anyway. So, the net profit,

[14:13]
max drawdown on all of these these

[14:14]
calculations can be made the exact same.

[14:17]
So, when you actually plug in your Pine

[14:19]
Script, it's going to it's going to work

[14:21]
the same anyway, and you just have

[14:23]
better better settings instead of having

[14:25]
to sit here, you know, opening your

[14:26]
indicator, tuning this one by one. You

[14:28]
know, there for Wolf Pack length, I

[14:30]
might want to tune from 1 to 100, and

[14:32]
then test all of these other three

[14:34]
variations against that. And then I have

[14:37]
to change all of them. So, there's just

[14:38]
going to be tens of thousands, if not

[14:40]
hundreds of thousands, of variations.

[14:42]
And we can when you can do 1,000 back

[14:44]
test per second to quickly rule out some

[14:46]
of those bad configurations so that your

[14:48]
strategy can really learn what works on

[14:51]
a certain coin, or what works on gold,

[14:53]
for example, or what works on NQ

[14:55]
futures. You can quickly find, you know,

[14:58]
the not only the best

[15:00]
the best max drawdown, for example, that

[15:03]
actually works on the data you're tuning

[15:05]
it for, which again in TradingView

[15:07]
custom date range, if you're doing this

[15:09]
properly, you would only tune for like

[15:11]
half the data or 60, 70% of the data,

[15:13]
and then configure your parameters,

[15:15]
saving 30% of the data to validate

[15:17]
afterwards to see to show unseen data to

[15:20]
your system. Make sure that, hey, does

[15:22]
the does it still have the same win

[15:23]
rate? Does it still have the same profit

[15:25]
factor or drawdown? Similarly, just to

[15:28]
just to make sure that you haven't

[15:29]
over-configured it. Cuz that's that is

[15:31]
the algorithm trading's worst fear is

[15:34]
configuring your strategy and

[15:35]
hyper-fixing it just for one data set.

[15:37]
So, that's why when you end up

[15:39]
randomizing the candlesticks, you are

[15:42]
doing these simulations with different

[15:43]
price action, different struct price

[15:46]
structures. And if your if your system

[15:48]
is flexible enough, then it will be able

[15:50]
to to respond to those well in real

[15:52]
time. For example, like going to some of

[15:54]
these bootstraps, let's just see how

[15:56]
well this this one performed. It didn't

[15:58]
collapse in the simulation that I'm

[16:00]
showing on screen. So, it was able to it

[16:02]
was still able to make 150% net profit,

[16:05]
and it was able to I mean, it once we

[16:08]
randomize the candlesticks, it didn't

[16:09]
just completely collapse to nothing and

[16:12]
die. So, this is why we do more of these

[16:14]
tests. Now, if you're if you're here on

[16:17]
YouTube and new to the channel, click

[16:19]
the like button. It really helps to

[16:20]
boost the video and this content, which

[16:22]
is so important for people to see and

[16:24]
know about. I'm offering this

[16:25]
information completely for free just on

[16:28]
YouTube. All you have to do is

[16:28]
subscribe, hit the like button, bell

[16:30]
notification, and I really I really want

[16:33]
to see people succeed in this industry

[16:35]
cuz retail traders are getting wiped out

[16:38]
by these trading bots, right? And you've

[16:40]
seen it in the betting markets. Betting

[16:42]
markets are they're mostly algorithms.

[16:45]
The algorithms are still wiping people

[16:47]
out in in in sports betting or in

[16:50]
trading. And so, it you know, showing

[16:52]
people how these industries build these

[16:55]
professional indicators and build these

[16:57]
professional trading systems is what I

[16:58]
specialize in. So, if you really are

[17:01]
liking this content, you know, comment

[17:03]
down below, and I'll be sure to answer

[17:05]
any question you guys have in the first

[17:07]
24 hours or so after I publish the

[17:09]
video. I try to reply to most comments.

[17:11]
If you see that's a trend, I want this

[17:12]
to be a back-and-forth thing with the

[17:14]
community. So, you can comment down

[17:15]
below. And also, if you're not

[17:17]
interested in building a whole trading

[17:19]
system like this for yourself, then I

[17:21]
have a website called signalswap. Then I

[17:24]
have a website that suits your needs.

[17:27]
This has a backtesting engine built into

[17:29]
it. It's called signalswap.io

[17:31]
and it's currently in alpha testing. So,

[17:33]
if you want to test this system, we have

[17:36]
a backtesting engine here that you can

[17:38]
just plug in your TradingView directly.

[17:40]
You don't have to convert it into Python

[17:42]
or any other programming language. So,

[17:43]
if you have your favorite Pine Script

[17:45]
indicators or

[17:46]
it has to be a strategy, plug it in

[17:47]
here, send it through the backtesting

[17:49]
system. It's currently uh down for a

[17:51]
little bit, just getting the server up

[17:53]
and running, but it is it is actually

[17:55]
working. So, should be live soon. And if

[17:57]
you want to test it, the website's in

[17:59]
alpha testing, so you need special

[18:01]
access. So, come to the free Discord

[18:03]
server. Uh the link to the Discord

[18:05]
server is down in the description, but

[18:06]
just shoot me a ping and I will give you

[18:09]
access to the instructions for, you

[18:12]
know, how to access the website and what

[18:13]
we need testing for. So, currently we

[18:15]
offer it's it's offering automated

[18:17]
trading for free, so you can plug in

[18:18]
your signals from Bybit or Apex and you

[18:22]
can plug in and as well Pine X as well,

[18:25]
which you've heard a lot on this

[18:26]
channel. And you just plug in your

[18:27]
trading signals from TradingView all the

[18:30]
way to the exchange and then you can

[18:31]
look at your metrics here and then also

[18:34]
publish them on the on the uh trading

[18:37]
bot marketplace as well, which only uses

[18:39]
real data. So, you can set up your

[18:41]
configuration here, you can monitor your

[18:43]
bot's performance, look at the trade

[18:45]
history, for example, and then once you

[18:48]
have some good metrics or results, it

[18:50]
will unlock a publish to marketplace

[18:51]
button here. And yeah, you can publish

[18:54]
it to the marketplace, which is only it

[18:55]
only uses real data. So, I I don't want

[18:59]
the I want to limit the backtesting

[19:01]
because backtest can back obviously be

[19:03]
misleading. That's obviously why we're

[19:05]
doing all these robustness tests, you

[19:07]
know, using our trading engine here. But

[19:09]
with the with with the system, uh 100%

[19:13]
real signals we've gathered and then it

[19:14]
shows the real metrics that it did get

[19:17]
from those, but it will have the

[19:19]
robustness back tests that is through

[19:21]
the website as well attached to that to

[19:23]
see did it pass the Monte Carlo

[19:25]
simulations, did it pass the out of

[19:27]
sample testing. So you can see the real

[19:29]
data up front and then if you're curious

[19:31]
about the back end here, it's not

[19:32]
required to attach these if you're

[19:33]
publishing to marketplace, but obviously

[19:35]
it goes a long way. You know, you'll get

[19:37]
a little badge and you get the you get

[19:38]
the dashboard here so people can see if

[19:40]
your bot really held

[19:42]
held up over stress tests as well. So

[19:45]
that is the that is the concept of the

[19:47]
website and it's free to list on the

[19:48]
marketplace. The creator of it keeps the

[19:50]
majority of the share of revenue from

[19:53]
the trading bots. So it's a win-win for

[19:55]
everyone because you're there is good

[19:58]
bots on the marketplace, quality bots. A

[19:59]
lot of these marketplaces you don't know

[20:01]
what you're getting, but on Signal Swap

[20:02]
you do. So signalswap.io is going to be

[20:05]
launching very very soon and you can

[20:06]
become an alpha tester. If you want to

[20:08]
get your bot, if you have a really good

[20:09]
trading system, you want to put it on

[20:10]
the marketplace, then become an alpha

[20:12]
tester and connect your signals up and

[20:14]
you can be one of the first to publish

[20:16]
in the marketplace when we officially

[20:18]
launch. It's also really easy to just,

[20:20]
you know, link your TradingView signals

[20:22]
as well. It's just a simple two-step

[20:25]
system here that we have. You just

[20:26]
select the exchange and then configure

[20:28]
how much money you want to put into the

[20:30]
signal when it does trade. Really easy

[20:32]
to create it and then manage it through

[20:34]
your centralized dashboard on Signal

[20:37]
Swap. So that's my shameless shout out

[20:39]
for Signal Swap. I'm really proud of

[20:41]
these of the system that I that we've

[20:44]
built. Everything is encrypted and so

[20:47]
it's almost ready to go. So hit the

[20:50]
subscribe button on the channel and I

[20:52]
think that's all for the video except

[20:54]
other than the fact that, you know, in

[20:56]
in the future I might be doing more of

[20:58]
these. You know, we're finding this

[20:59]
trading system, showing you guys kind of

[21:01]
where Claude took it overnight and it's

[21:04]
really just like one-stop shop

[21:07]
overnight. So I will set my prompt. I'll

[21:09]
ask it to test various systems and then

[21:11]
keep refining and and retaining the

[21:14]
trading systems that work and discarding

[21:16]
the ones that obviously don't. And so,

[21:19]
so this is a far better method. You

[21:20]
know, a lot of people will just scour

[21:23]
maybe the TradingView marketplace for

[21:24]
good bots, but you don't really know are

[21:26]
those bots robust? Do they pass stress

[21:28]
tests? What are the trading

[21:30]
configurations for them? Now, this will

[21:31]
take this takes care of all of that for

[21:33]
you as well. It just picks It picks the

[21:35]
best metrics, uh

[21:37]
finds the best metrics parameters, and

[21:39]
it shows you the stress test results, as

[21:40]
well as Signal Swaps backtesting does

[21:43]
the exact same thing. It shows you the

[21:45]
the full in-sample out-of-sample

[21:46]
comparison here, as well, that is

[21:48]
configured for. So, that is going to be

[21:50]
the end of today's video. I'll see you

[21:52]
in the next one, as well, and maybe we

[21:55]
can I can show you how to fully set up

[21:57]
the Monte Carlo simulations or the

[21:59]
VectorBT simulations, as well, just so

[22:02]
that you guys can run these on your

[22:03]
machine, as well. So, uh there's a bunch

[22:05]
of other videos on the channel you could

[22:07]
check out, all with similar uh

[22:08]
methodology, and I hope that this really

[22:11]
levels up your game for building trading

[22:14]
systems. If you're new to algorithmic

[22:16]
trading, it really is a huge black hole

[22:20]
of knowledge to gain, but I hope to help

[22:23]
make that a little bit easier for all of

[22:25]
you. So, I'll see you in the next video.

[22:27]
Thank you.