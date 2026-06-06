[0:00]
All

[0:00]
right, let me set the context for the

[0:02]
video. In this video, we're giving three

[0:04]
AI agents $1,000 to trade with. They'll

[0:06]
be trading on cryptocurrency markets.

[0:08]
The AI agents will be given u historical

[0:11]
data about the stock, how it is

[0:13]
performing, how's the price going up and

[0:14]
down, and a bunch of financial

[0:15]
indicators. Based on this, the AI agent

[0:18]
can autonomously take positions. It can

[0:20]
either long or short a specific

[0:22]
cryptocurrency. And based on that, over

[0:25]
time, we'll see if the final amount of

[0:27]
money that it makes u is more than

[0:29]
$1,000 or less than $1,000. This agent

[0:31]
has been running for me for the past one

[0:32]
day. And this has this is how it's

[0:34]
performed until now. Um if you go to

[0:36]
aitrading.hunxabs.com,

[0:38]
you'll find how much money the three

[0:41]
agents have right now. They started with

[0:42]
$1,000. Cloud right now has 18, so it's

[0:45]
up $18. Deepseek is down around $30.

[0:47]
Quen is up around $50. Overall, we're in

[0:49]
the profits and it's traded fairly well.

[0:51]
All three of them have traded around

[0:52]
half a million dollars of u

[0:54]
cryptocurrency until this point. Um if

[0:56]
you want to deeply see uh what exactly

[0:58]
is happening under the hood. Um and you

[1:00]
know what prompt is going to the LLM,

[1:02]
how is it responding um and what kind of

[1:04]
positions it's taking. Um you can also

[1:05]
track them over here. This project is

[1:08]
inspired from Alpharena which came

[1:10]
around 2 weeks ago. I think two weeks

[1:12]
ago they gave $10,000 to six models to

[1:15]
train with. Um, and if you look at their

[1:16]
performance, again, the Chinese models

[1:18]
are the ones that are performing the

[1:19]
best, which begs the question, do the

[1:21]
Chinese know something we don't, or are

[1:22]
they their models trained a certain way

[1:24]
with, you know, trading data? I think

[1:26]
the answer is no. Um, but again, there

[1:28]
could be some speculation because a lot

[1:29]
of these teams came out of HFTs. Um,

[1:32]
generally though, I would not recommend

[1:33]
any of you try to trade with AI. I think

[1:35]
we're just lucky over here. That's my

[1:36]
personal feeling. Um, I'm actually very

[1:38]
surprised by the final um, outcome that

[1:40]
came over here and the fact that I've

[1:41]
not lost all the money. Um so with that

[1:43]
information I mean choose what you would

[1:45]
like to do. Um that's enough context. If

[1:47]
you want to learn or the things that

[1:49]
you're going to learn over here are

[1:50]
mostly going to be financial indicators.

[1:51]
Basically a bunch about finance. How

[1:53]
exactly do traders in the real world uh

[1:56]
take specific decisions on whether to

[1:58]
buy a stock or to sell a stock purely

[2:00]
based on the existing prices or the

[2:03]
price data. Um not based on any news or

[2:05]
any other external factors or macro

[2:07]
factors. Um that's what these guys do as

[2:09]
well. uh if you want to build it from

[2:10]
first principles you should all this

[2:12]
inspiration and you know all the model

[2:14]
prompts and financial indicators that I

[2:16]
have taken I've taken from you know by

[2:18]
reading through their prompts and what

[2:19]
they're doing um in terms of the text

[2:21]
stack it's fairly simple bunes lighter

[2:23]
is the uh exchange that we're using and

[2:25]
the three lms that we're using u are

[2:28]
quen deepseeek and claude um open router

[2:31]
for sending requests to the lm as the

[2:33]
backend/runtime in which we're running

[2:35]
our code and versi to talk to the lm

[2:38]
lighter SDK to talk to lighter. That's

[2:40]
enough context. I'll see you guys in the

[2:42]
video.

[2:45]
So, recently a new project called Alpha

[2:48]
Arena got released. And this project

[2:50]
basically gave $10,000 each to six AI

[2:54]
models to trade. This video we're going

[2:57]
to look at what exactly they're doing,

[2:59]
what sort of prompts are they sending to

[3:01]
the LLM. Um how is the LLM responding?

[3:04]
How is it taking trades? And overall in

[3:06]
the past few days um they've not done

[3:09]
too bad. I mean if you average out I

[3:11]
think u in the end they've not made any

[3:13]
money not lost any money. Um Quinn and

[3:16]
basically the Chinese models have been

[3:17]
doing really well. Um which is

[3:19]
surprising like why would DeepS do well?

[3:21]
Is it because they were created by NHF?

[3:22]
I don't think so. Um but that's what's

[3:24]
happened until this point. Um you can

[3:27]
actually track everything that's

[3:29]
happening. Uh what kind of prompts are

[3:31]
going to the LLM? how is CLM responding

[3:34]
back with positions to take and those

[3:36]
are the exact positions that the model

[3:37]
ends up taking. Um over the last oh it's

[3:41]
been around 10 days uh maybe 7 days um

[3:44]
this has been the performance um we'll

[3:46]
try to create something similar we'll

[3:47]
create uh a trading sort of a view like

[3:51]
this u which shows how multiple models

[3:54]
are performing um when we give them

[3:56]
prompts very close to the prompts that

[3:58]
are given over here. So if you look at

[4:00]
uh the model chat, it pretty much has

[4:03]
everything that's being forwarded to the

[4:04]
LLM. Um it tells it it's been these many

[4:07]
minutes since you started trading. Um

[4:08]
and this is the current time and these

[4:10]
many trades have been invoked.

[4:15]
This is the time and you've been invoked

[4:16]
these many times. Um that's 1200 times

[4:19]
since the beginning. Um bunch of other

[4:22]
metadata, the current price, uh a bunch

[4:25]
of financial indicators that we'll learn

[4:27]
about. uh some more financial indicators

[4:30]
short-term and long-term. Um and then

[4:32]
the same data for Ethereum, same data

[4:33]
for Salana, same data for a bunch of big

[4:35]
coins. Um so these are all crypto coins.

[4:37]
We forward them um the last let's say 3

[4:40]
days of data and the last few hours of

[4:42]
data. Uh and some financial indicators

[4:45]
related to that data. Um that's all the

[4:47]
LM has access to. The LM then responds

[4:49]
back with what kind of positions it

[4:51]
wants you to take. Does it want you to

[4:52]
buy u or sell a specific coin? Um

[4:55]
specifically here we're not buying or

[4:56]
selling. We're longing or shorting. So

[4:58]
we'll talk a bit about um what is a long

[5:01]
and a short. What are perpetual futures?

[5:03]
Um and how are they different from

[5:04]
normal trading or spot trading that you

[5:06]
might be a little more comfortable with.

[5:08]
Um that's all the context for this

[5:09]
video. We'll kick things off. U let me I

[5:13]
created slides for these um for this

[5:15]
video. Um they're not very

[5:17]
comprehensive. I left them in the

[5:18]
middle. But let me see what I have

[5:20]
written. Preface

[5:22]
Alpha is a project u that's doing fairly

[5:25]
well. Uh this is from a few days ago

[5:27]
when I realized I want to make this

[5:28]
video. Back then um Deep Seek was at the

[5:31]
top with the $13,000 compared to the

[5:34]
starting $10,000.

[5:36]
Um CZ made a tweet. CZ is the founder of

[5:39]
Binance and I sort of agree with him. I

[5:41]
don't think necessarily anything would

[5:43]
win over here. U I think it's pretty

[5:45]
random and you can see that in the final

[5:47]
outcome as well. Um, in the final

[5:49]
outcome, I think if you average them

[5:50]
out, every every model is, you know,

[5:51]
they still have $50,000, which is what

[5:53]
they started with. Um, I think this is a

[5:55]
little bit of luck. Um, because if you

[5:58]
know the stats, 5% of people actually

[6:02]
make money in trading. Um, and I don't

[6:04]
think LLM have that kind of knowledge

[6:07]
yet. Um, even if they do, then everyone

[6:09]
would start trading with LM and then

[6:10]
there no there would be no real alpha.

[6:12]
Everyone would just buy the same

[6:13]
positions and sell the same positions.

[6:14]
um if it becomes open knowledge which is

[6:16]
why I don't think and I agree with u see

[6:19]
over here a counterargument could be

[6:20]
made that if enough people use the same

[6:22]
AI then it's buying power will push the

[6:24]
price up by itself um and vice versa so

[6:25]
if the I thinks the price needs to go up

[6:28]
it'll already be priced in uh and by the

[6:30]
time you buy um the price is sort of

[6:32]
normalized with that information um

[6:34]
anything else that I have over here uh a

[6:36]
bunch of disclaimers uh this is not

[6:38]
financial advice and this probably is

[6:40]
not going to work by the time you see

[6:41]
this video you probably are going to see

[6:43]
one or two days worth of tearing data

[6:45]
and how it performed. I'll create a

[6:46]
public dashboard where you can track how

[6:49]
it's doing. Um but high probability that

[6:51]
this is all luck and um there's no real

[6:55]
uh game to play here or no real script

[6:58]
to run that will actually make you

[6:59]
money. What we're building um is

[7:02]
eventually going to be a simple script

[7:03]
in the end. Um a script where we fetch a

[7:07]
bunch of data from an exchange. Um we'll

[7:09]
decide what exchange to pick and where

[7:10]
we're actually placing trades in the

[7:12]
end. Um but there would be a

[7:14]
decentralized or centralized perpetual

[7:16]
future exchange um for which we'll be

[7:18]
fetching a bunch of data. So what would

[7:21]
this data look like? Um for these guys

[7:23]
um the data pretty much looks something

[7:26]
like this. U it's just a candlestick

[7:29]
data for the last few hours and the last

[7:31]
few days.

[7:33]
compressed in a specific format and uh a

[7:35]
bunch of indicators, financial

[7:37]
indicators that are used fairly

[7:38]
commonly. Um so nothing that nothing too

[7:42]
crazy like the indicators you might not

[7:44]
have heard of. I also did not haven't

[7:46]
heard of a bunch of these indicators,

[7:47]
but they're easy enough to write. Um and

[7:50]
generally when you write trading

[7:51]
algorithms uh without an AI, you sort of

[7:53]
use these indicators to figure out if

[7:55]
two indicators sort of converge is when

[7:56]
the price will go up or price will go

[7:58]
down. Um all of that is left to the AI.

[8:00]
Our job is to just gather this

[8:02]
candlestick data. Let's say every 5

[8:04]
minutes. Um tell the LLM this is our

[8:06]
current position. This is how much money

[8:08]
we have made until this point. This is

[8:09]
the candlestick data for the last 4

[8:12]
hours and the last few days. Basically a

[8:14]
long-term and a short-term sort of a

[8:15]
trend. Um and the LM will respond back

[8:17]
with if it needs to update any of the

[8:19]
trades or not. [clears throat]

[8:22]
So in the end a back end that is going

[8:23]
to talk to the LM and the LM is going to

[8:25]
tell us what to do and we're going to

[8:26]
place those orders on the exchange.

[8:28]
We'll also be reading a bunch of data

[8:29]
from the exchange. All of this would be

[8:31]
HTTP. So all the connections between our

[8:33]
back end and the exchange would be an

[8:35]
HTTP uh request. All the requests over

[8:38]
here are going to be HTTP requests.

[8:39]
Standard LM calls like you might have

[8:41]
done until this point. Um there's

[8:42]
nothing too fancy over here. I think by

[8:44]
the end this should be less than 200

[8:46]
lines of code, maybe five files. Um so

[8:48]
nothing crazy to write. Uh when it comes

[8:50]
to the code um one thing you need to

[8:52]
understand is what are pers and how are

[8:55]
they different from spot exchanges

[8:57]
before we start um

[8:59]
the reason we're trading on ps is

[9:01]
because these guys are trading on ps um

[9:03]
if I originally thought we'd trade on

[9:05]
spot but before I throw these terms at

[9:08]
ter terms at you um if you scroll to the

[9:11]
top of backpack which is also an

[9:13]
exchange you'll see two things um spot

[9:15]
and futures um spot is a traditional

[9:18]
exchange you might be comfortable with

[9:20]
until this point, you might have heard

[9:22]
of spot. Uh, and if you haven't, if

[9:24]
you're ever placing a trade on Serata

[9:26]
like you and I do, um, then for example,

[9:29]
if you're buying five stocks of uh, Axis

[9:32]
Bank, you're basically trading u against

[9:34]
a counterparty. You're giving them some

[9:36]
INR, you're taking some access bank

[9:37]
stock with them. That's what happens

[9:38]
when you go to backpack.exchange.

[9:41]
Oh sh What is that?

[9:45]
That's what happens when you go to

[9:46]
backpack.exchange/trade/sour.

[9:47]
exchange/trade/soul_usd.

[9:49]
Um, so if I try to buy one soul and I

[9:52]
click on the buy button,

[9:55]
um, not fulfilled because I don't have

[9:57]
enough funds, which is weird or I

[9:59]
probably don't have enough USD. I

[10:00]
already have a lot of soul. So, I can

[10:03]
sell some Solana and show you. If I

[10:04]
click on sell and click on one, um, I

[10:06]
was able to sell one Solana. Now, my

[10:08]
Solana balance has gone down and my US

[10:10]
order balance has gone up or has it?

[10:12]
Yeah, it has 192. Um but

[10:16]
if you go to futures u the thing that

[10:18]
we're doing is slightly different. It's

[10:20]
actually significantly different from

[10:22]
what happens on spot. Um the price that

[10:24]
you see over here is also significantly

[10:26]
different from the price that you will

[10:28]
see in on the per market. Um the reason

[10:32]
for that is fairly complicated. For that

[10:33]
you need to understand how pers work

[10:35]
under the hood. Why do they even exist

[10:37]
in the first place? And what is the

[10:38]
difference between a spot and a

[10:40]
perpetual future? high level um per pups

[10:44]
are future contracts that they're closer

[10:45]
to option contracts but they don't have

[10:47]
any expiry which is the technical

[10:48]
definition for it u for that you need to

[10:51]
understand what are future contracts um

[10:53]
if you've ever traded on zerodha as a

[10:56]
degenerate not as normal retail users um

[11:00]
you might already know what futures are

[11:01]
in fact 90% or 95% of zeroda's revenue

[11:05]
actually comes from futures and not the

[11:07]
trades that you and I make um so it's

[11:09]
it's basically it's close to gambling Um

[11:12]
it's when you

[11:14]
I quickly take you through it. U let's

[11:16]
create a few flowcharts here. What we

[11:18]
need to understand is the difference

[11:19]
between spot

[11:23]
um

[11:24]
options trading and then per

[11:29]
spot I hope you understand by now. If

[11:31]
you go to

[11:31]
backpack.exchange/soultrade/soloulusd

[11:35]
what you're doing over here when you

[11:36]
click on buy or sell is a spot exchange.

[11:38]
Um if you go to zeroda.com and try to

[11:40]
buy 10 access bank stocks, you're

[11:42]
basically trading on a spot exchange.

[11:44]
Options are a little more tricky. Um and

[11:47]
this is where people start to gamble a

[11:48]
little bit. Options is when you play pay

[11:50]
a really small price, for example, 200

[11:52]
rupees. Um to buy the option or to get

[11:56]
the option of buying a stock, let's say

[11:59]
200 access bank stocks at a specific

[12:02]
time which could be today's 25th

[12:04]
October. So let's say 27th October. If I

[12:06]
feel that the price of Access Bank is

[12:08]
going to go down, I will pay someone a

[12:11]
premium. I will pay someone 200 rupees

[12:14]
which is also called a premium and tell

[12:17]
them hey I would like to buy 200 access

[12:20]
stock banks from you on 27th October or

[12:24]
before that um but I would really like

[12:26]
to buy them. Um I also have the option

[12:28]
of not buying them. So I may or may not

[12:30]
buy 200 Access Bank stocks from you. the

[12:32]
other counterpart party immediately gets

[12:34]
200 rupees and if the price of access

[12:36]
bank drops enough um then it becomes a

[12:39]
profitable trade for me because I get

[12:40]
200 access bank stocks from them um at a

[12:43]
much cheaper price at which I would have

[12:45]
bought them today. So if the access bank

[12:47]
stock basically drops by 1 rupee u I'll

[12:50]
be able to buy them at

[12:53]
sorry my bad if the access bank stock

[12:55]
goes up by 20 rupees I can still buy

[12:58]
access bank at the price on this day so

[13:01]
uh

[13:02]
is that true I think that's true

[13:09]
I think it's at a specific price

[13:12]
yeah I have to Google this I get the

[13:15]
option of buying 200 200 access stock

[13:17]
bank uh stocks 200 access stocks at a

[13:21]
specific price let's say 200 rupees per

[13:23]
stock at a specific date let's say 27th

[13:26]
October that is what an option is I can

[13:28]
go to zera.com I can buy an option I can

[13:31]
pay 200 rupees abi and then I can sort

[13:33]
of see my position go up and down based

[13:35]
on the price of access bank because I

[13:37]
told the other party I can I'll buy 200

[13:39]
access bank stocks from you at 200

[13:41]
rupees per stock if the price goes up I

[13:44]
can still buy it at 200 rupees per

[13:46]
stock. So that's where the profit comes

[13:48]
from. If the price of Access Bank

[13:50]
tomorrow goes up to 300 rupees, then I

[13:53]
can buy Access Bank at 200 rupees. So I

[13:56]
I can spend 200 into 200 that's 40,000

[13:59]
rupees. um and get back 200 access

[14:02]
stocks and since the current price is

[14:03]
300 I can immediately send it for sell

[14:05]
it for 60,000 rupees um and and you know

[14:09]
uh basically convert my 200 access bank

[14:11]
stocks into 60,000 and make a profit of

[14:15]
20k um so with a premium of 200 rupees I

[14:17]
was able to make 20,000 rupees um

[14:20]
provided the price of access bank went

[14:22]
up um the premium of course is um sort

[14:24]
of priced in the way you sort of you can

[14:28]
only make I mean it's of course not as

[14:29]
simple as this like it won't be 200

[14:31]
rupees and the net outcome will won't

[14:32]
very easily be 20,000. Of course, the

[14:34]
price of access bank will not go up from

[14:35]
200 to 300 in 3 days. U but you get the

[14:38]
idea. Um these are more technically

[14:40]
contracts between two parties where one

[14:42]
party gives the other party the option

[14:45]
not the obligation but the option of

[14:46]
buying a stock at a certain price at a

[14:48]
specific date. Buying a stock at a

[14:51]
certain price at a certain date and the

[14:54]
other party gives them a premium. Now if

[14:56]
the price is in the favor of this party

[14:58]
then they just keep the 200 rupees. The

[15:00]
first party never really exercises their

[15:02]
option. Um but if it becomes favorable

[15:04]
for the other party then they exercise

[15:05]
the option they take the 200 access

[15:07]
stock um stocks from them and you know

[15:10]
sell them or a lot of times if the price

[15:12]
of access bank does go up the price of

[15:14]
this uh future will also go up to 500

[15:18]
1,000 20,000 you know let's say 15,000.

[15:21]
So I can also just sell this option to

[15:24]
someone else and then that other person

[15:25]
can eventually finish the trade on 27th

[15:27]
October. Today's 25th tomorrow it might

[15:28]
happen that the price of this option

[15:30]
goes up because the access bank price is

[15:32]
going up. Um so I can immediately sell

[15:33]
the options. Someone else then finally

[15:35]
takes the final trade and you know um

[15:37]
waits for 27th October to come and end

[15:39]
it. Um that's what options are. We're

[15:40]
not dealing with options today. Just

[15:41]
wanted to share a brief history about

[15:43]
what options are and how are they

[15:44]
different from um perpetual futures. Um

[15:48]
hopefully we understand spot, we

[15:50]
understand options. Pers are a I

[15:53]
wouldn't call it a new category. I think

[15:54]
they're pretty old. 2008 or 2012, I

[15:57]
forgot is when they were introduced. Um

[15:59]
and they are almost an option only. Um

[16:03]
but they don't have an expiry date. So

[16:06]
they you could consider them to be very

[16:07]
close to what you saw on the left. Um

[16:09]
but they do not have an expiry date.

[16:12]
Which means when you buy a per contract

[16:15]
um you're pretty much saying I'm willing

[16:16]
to buy or sell something at a specific

[16:18]
price with no expiry um and then the way

[16:22]
the contract sort of continues with this

[16:24]
specific property um makes the price of

[16:27]
a perpetual contract um which is you

[16:30]
know this thing right here sort of

[16:32]
follow the price of the actual

[16:34]
underlying asset. So if this is soul

[16:37]
USDC per and this is soul USD um this as

[16:40]
you can see is sort of following very

[16:42]
closely the price of this guy um because

[16:45]
the way the contract is sort of written

[16:48]
between two parties um as you buy and

[16:50]
sell this contract um there is something

[16:53]
called a funding rate that one party has

[16:55]
to give to the other party. People who

[16:56]
are buying have to give to the people

[16:57]
who are selling or people who are

[16:59]
longing have to give to the people who

[17:00]
are shorting. Um if you add that up uh

[17:05]
or you know the funding rate sort of

[17:06]
makes people buy a specific side more

[17:10]
heavily. The funding rate makes sure

[17:12]
people are aggressively buying one side

[17:13]
or the other side. Um because at a

[17:15]
specific time which right now is 2

[17:17]
minutes from now um people from this

[17:20]
side are going to pay the people from

[17:21]
the other side. People from the longs

[17:23]
are going to pay the shorts or vice

[17:24]
versa based on whether or not this

[17:25]
funding rate is positive or negative. Um

[17:28]
because of this 1 hour counter and you

[17:31]
know one party paying the other party,

[17:33]
people aggressively buy on one side to

[17:35]
make sure that the final price um is

[17:37]
close to the final price over here. Um

[17:39]
the final price of the perpetual

[17:41]
contract is close to the final price of

[17:43]
the actual underlying asset. Uh happens

[17:45]
because of this thing called the funding

[17:46]
rate. U the funding rate sort of ensures

[17:49]
okay if the price deviates too much. If

[17:50]
people buy the contract too much and

[17:52]
this price deviates too much, the other

[17:53]
side has the incentive of sort of

[17:55]
selling um bringing the price closer to

[17:58]
this mark price that you see over here

[17:59]
which is the latest I don't know if this

[18:02]
is the mark price or this one the index

[18:04]
price which is the actual price of the

[18:06]
underlying asset. So as long as this is

[18:09]
close to this we'll be good. If this is

[18:11]
not close to this is when one part is

[18:12]
incentivized um to heavily buy or sell

[18:14]
or you know heavily long or short uh

[18:16]
which makes this price come closer to

[18:19]
the actual underlying index price. Um so

[18:21]
it is a contract. There is no real stock

[18:24]
involved which is why pers are super uh

[18:28]
popular for assets that are very hard to

[18:31]
get. Good example is uh a Tesla stock or

[18:34]
a Coinbase stock or a any stock in the

[18:38]
US market uh any stock in the stock in

[18:39]
the Japanese market or crude oil. Um

[18:41]
these are things that you and I can't

[18:43]
really buy easily. It's very hard for

[18:44]
Indians to buy u a stock of meta but we

[18:48]
can buy a perpetual contract of meta. U

[18:51]
we can long or short meta thanks to

[18:53]
exchanges like these. uh while there is

[18:56]
the market makers are constantly making

[18:57]
sure that the price of this perpetual

[18:59]
contract remains close if not exactly

[19:01]
the same as the price of the of real of

[19:03]
the real meta stock. Um the downside is

[19:06]
you and I never actually own um the meta

[19:08]
stock. So there's something to keep in

[19:10]
mind. Um if you're ever buying of

[19:11]
perpetual contract, you are buying the

[19:13]
contract and not the underlying asset.

[19:15]
So if all the meta holders get a

[19:17]
dividend, you do not get it. Um if all

[19:19]
the meta holders get to uh vote

[19:21]
somewhere um you do not get it because

[19:23]
there's like limited stocks of meta

[19:25]
right there like let's say 100k stocks

[19:27]
of meta that's distributed amongst a

[19:29]
bunch of people employees or otherwise

[19:31]
um it might happen that the price of

[19:34]
these 100k stocks or the market cap is

[19:36]
let's say 1 trillion um but there might

[19:39]
be perpetual contracts of 10 trillion um

[19:41]
trading on top of this 1 trillion market

[19:44]
cap stocks. This is the actual

[19:47]
underlying stock and there might be a

[19:48]
market on it where people are actually

[19:50]
trading with each other much bigger

[19:52]
volumes than the actual market cap of

[19:53]
the company itself. Um this is why pers

[19:56]
are a little common. Um the other reason

[19:58]
they're very common is it's very easy to

[19:59]
introduce leverage trading in per if you

[20:01]
ever want to do a 1x 2x 10x. For

[20:04]
example, I only have $8,000 in my

[20:06]
backpack wallet, but I can actually

[20:07]
create a position where I'm trading with

[20:09]
$71,000 uh because I'm taking a 10x

[20:11]
leverage on my position. Um what that

[20:13]
does is if the price goes up by 1% u my

[20:16]
actual profits will go up by 10% if I've

[20:18]
taken a long position. But if the price

[20:19]
goes down by 1% then I my actual profits

[20:21]
will or my loss will be amplified. I'll

[20:24]
lose 10% of my position. If the price

[20:27]
goes up by 10% I'll make 2x because I

[20:30]
have a 10x leverage. But the but if the

[20:32]
price goes down by 10 uh 10% then I sort

[20:34]
of lose my position and [clears throat]

[20:36]
I lose $7,000 not $70,000 but it goes do

[20:40]
you get the idea? the B the base capital

[20:41]
the available equity that I have is

[20:43]
$7,000 which is why even though I'm

[20:46]
trading uh on paper with $72,000. Um if

[20:50]
it goes up by 10% I will double my base

[20:53]
amount. So my $7,000 becomes 14 $14,000.

[20:55]
But if the position goes down by 10% or

[20:57]
the price goes down by 10%. Uh I

[20:59]
basically am down to zero. So I I

[21:01]
basically have no money by the end.

[21:02]
Hopefully um that's a decentish idea of

[21:05]
what pops we don't need to understand

[21:06]
any of the history u because it's very

[21:09]
easy to just understand what they're

[21:10]
doing um and you know take these

[21:11]
positions for example if I if I take a

[21:14]
long um with let's say $18,000 over here

[21:17]
um then I have this sort of open

[21:19]
position

[21:21]
which is currently down $5. Um if the

[21:23]
price climbs down I'll keep on losing

[21:26]
more and more money but if the price

[21:27]
climbs up I'll make more money. Um I'm

[21:29]
going to close this position. Um but you

[21:31]
get the idea. I just lost $4 on a

[21:33]
perpetual contract not on the actual uh

[21:37]
Solana uh

[21:40]
market or you know I did not actually

[21:42]
buy Salana or sell Solana I actually

[21:43]
just traded a contract uh which made me

[21:45]
lose some money right now. Um so if you

[21:47]
look at my order history you'll see one

[21:49]
order where I lost um you know whatever

[21:54]
I don't see the actual P&L here uh

[21:56]
position history.

[21:58]
There you go. I'll actually made some

[22:00]
money by the end. By the time I ended up

[22:02]
trading or you know my position ended up

[22:04]
closing um the price moved and I

[22:06]
actually made $1, which is good, but I I

[22:08]
also lost a lot of money in fees because

[22:10]
it was a pretty big position. And if you

[22:12]
multiply that by the trading fees, uh

[22:14]
wow, it's like $7. That's insane. Um so

[22:16]
I made some profit on paper, but the

[22:19]
trading fee sort of took away around $8

[22:20]
for me, which is not great. Um that's

[22:22]
why you never really open a and close a

[22:24]
position because your position is so

[22:25]
leveraged. Um, I'm taking a position of

[22:28]
$18,000. So, the fees on this becomes

[22:30]
really high. 01% is a very big number if

[22:32]
this is what you're trading with. Um, so

[22:34]
these are things to keep in mind keep in

[22:36]
mind when you're placing these trades.

[22:37]
Um, but hopefully you get the idea of

[22:40]
what perks are. Um, and what's going to

[22:44]
happen in the end is all of these trades

[22:45]
are going to be taken by the AI and not

[22:47]
me the human. That should probably cover

[22:50]
everything I had in the slides. From

[22:51]
here is where we start to code. Um there

[22:54]
are some financial indicators that we

[22:55]
need to read about or know about. U a

[22:58]
bunch of these financial [clears throat]

[22:59]
indicators

[23:05]
are present here in the model chart. So

[23:07]
if I open the chart, you'll see u an EMA

[23:11]
which is an exponential moving average

[23:13]
EMA 20 um mid price which is basically

[23:16]
the current price at a specific

[23:18]
interval.

[23:20]
Exponential moving average, MACD, RSI,

[23:22]
these are all financial indicators.

[23:24]
We'll see how many of these we finally

[23:25]
code. Um but at least for these guys

[23:27]
they provide these four financial

[23:29]
indicators for a buy minute data which

[23:32]
basically means you know what happened

[23:34]
at a 1 minute level for the last let's

[23:37]
say 20 candles. Um did it go up? Did it

[23:39]
go down and what are the financial

[23:40]
indicators for these last 20 minutes? Um

[23:43]
and then they have a 4hour sort of a

[23:45]
candle. Um what happened for the last 20

[23:48]
candles in these 4hour candles. So this

[23:51]
one candle is 4 hours of data. This

[23:53]
other candle is 4 hours of data. So what

[23:55]
happened in the last 4 into 20 hours and

[23:58]
what happened in the last 4 into 20

[23:59]
minutes um is the data that leaks I

[24:01]
finally forward um to the LLM very close

[24:05]
to what's happening here. The LM

[24:07]
responds back with what positions to

[24:08]
take and we take those positions and

[24:10]
hope for the best. Um that's what we're

[24:12]
coding today with that. I think there's

[24:14]
one last slide before we start coding

[24:16]
this one right here. Um you can pick any

[24:19]
one of these exchanges to trade on. Um

[24:21]
there are decentralized purps, there are

[24:23]
centralized per um there is good reason

[24:27]
to choose one over the other. You can

[24:29]
pick any one. I am going with lighter

[24:31]
today. It's a per that I was sort of

[24:33]
experimenting with 2 days ago. Um and I

[24:36]
have actually ported over their Python

[24:39]
SDK. Uh

[24:43]
this is their Python SDK that lets you

[24:45]
trade using Python. I ported this over

[24:48]
to JavaScript today only. In fact, a few

[24:51]
hours ago. Um,

[24:54]
so this is what we'll use. This supports

[24:56]
almost everything we need today, which

[24:58]
is getting candlestick data, placing

[25:00]
orders, getting the positions, things

[25:01]
like these. Um, so I've basically

[25:03]
written an SDK that will make it very

[25:05]
simple for us to do what we want to do.

[25:06]
There are a bunch of examples and all

[25:08]
four of these examples is all we need.

[25:09]
Um, I should probably add more examples

[25:11]
here because there's a lot you can do in

[25:13]
an exchange. Today though we only need

[25:15]
to create orders which means hold a

[25:17]
position cancel order if the LM tells us

[25:19]
this position is going you know if

[25:20]
you've made a profit or if it's going

[25:21]
down too much it wants us to cancel it

[25:24]
get k lines which basically means get

[25:26]
these candles um for the last xyz hours

[25:29]
and uh get positions which tells the LLM

[25:32]
how much money have we made until now

[25:35]
and what all positions are open um are

[25:37]
we long on BTC right now are we short on

[25:39]
soul what positions the LM took in the

[25:41]
past is what we'll get from this fourth

[25:43]
sort an example. Um so in the end the

[25:45]
code should be fairly simple u and you

[25:47]
know these examples will sort of guide

[25:48]
us through it. With that let's get into

[25:50]
the next part. Let's understand uh

[25:52]
probably the text stack for the day.

[25:57]
Right. Let's talk through the text stack

[25:59]
for the day. We'll be using bun as the

[26:01]
runtime. Unfortunately, you won't be

[26:03]
able to use NodeJS because the bullet js

[26:06]
SDK that I've written um sorry the

[26:09]
lighter JS SDK that's so wrong lighter

[26:13]
uh unfortunately at some points uh

[26:17]
specifically over here

[26:22]
where is it probably here uses some

[26:25]
internals of bun so these will not work

[26:27]
with NodeJS um so unfortunately the SDK

[26:30]
that we're using is not compatible with

[26:32]
NodeJS. So you will have to use one as a

[26:34]
runtime. Versel AI SDK to connect to the

[26:37]
LLM and do a bunch of tool calls. Prisma

[26:39]
posgress as our database. React on X for

[26:42]
the front end. Open router as a router

[26:44]
layer that lets us connect to various um

[26:46]
LLMs. Should be fairly straightforward.

[26:49]
Less than 100 200 lines of code I'm

[26:51]
assuming. Um I'm assuming this to be a

[26:53]
black box and assuming u open router AI

[26:56]
SDK to be a black box. Shouldn't be a

[26:58]
lot of lines of code. Um I've written

[27:00]
down the information u a little more

[27:02]
elaborate information around what

[27:04]
financial indicators we'll be using

[27:06]
today. Um we'll be using mid prices,

[27:10]
EMA. Let me just name them. Mid prices,

[27:14]
EMA

[27:16]
and MACD as the three financial

[27:19]
indicators for the day. This is all the

[27:21]
information that the LM would have for a

[27:23]
short-term duration and for a long-term

[27:25]
duration. And based on these u

[27:27]
indicators it needs to make the final

[27:29]
decision of whether it needs to buy sell

[27:31]
uh long or short a specific asset.

[27:35]
Mid prices is the easiest one. Um this

[27:37]
is basically the current price at a

[27:39]
specific interval. So if you look at any

[27:42]
market, let me open [clears throat] the

[27:44]
Solana per market over here. If I select

[27:48]
1 minute candles, then the mid price

[27:51]
here would be 199. Mid price here would

[27:54]
be 199.96. Here would be 199.93. Here

[27:58]
would probably be somewhere in the

[27:59]
middle of this high and this low. So

[28:01]
199.84. Here it would be 199.66.

[28:05]
199.74. So on and so forth. So the mid

[28:07]
prices um for all of these last let's

[28:10]
say 20 candles or 10 candles. What is

[28:12]
the mid price between the high and the

[28:15]
low of the specific interval? Um it can

[28:17]
be high and low. For example, for this

[28:19]
candle, this is the high, the thinner

[28:21]
green line that you see here, and this

[28:23]
is the low. For this one, this was the

[28:25]
high, this was the low. Or it can be the

[28:27]
mid of the open and the close. So, this

[28:28]
is where the market opened. This is

[28:29]
where the market closed in this specific

[28:31]
candle. So, we can pick in either one of

[28:33]
these. We'll probably pick uh open and

[28:35]
close. So, this is where this market

[28:37]
opened. This is where this market

[28:38]
closed. The mid price is going to be the

[28:40]
mid price for that interval. If you take

[28:42]
that for the last 10 intervals, we get

[28:43]
the mid price for the last 10 intervals.

[28:45]
That's the easy one. um which is mid

[28:48]
prices. Um then comes something called

[28:51]
simple moving average. Um this is fairly

[28:55]
simple. Uh it's basically the average of

[28:57]
let's say the last 10 candles. Um so the

[29:00]
simple moving average for this and this

[29:03]
would be you know the midpric here plus

[29:05]
the mid price here plus the mid price

[29:07]
here so on and so forth divided by the

[29:09]
number of candles. So just like a moving

[29:10]
average um for the last you know n

[29:13]
intervals. So SMA 20 would be the last

[29:15]
20 intervals car simple moving average.

[29:17]
I can find the SMA 20 for this candle.

[29:20]
That will be the last 20 divided by 20

[29:23]
of this candle. That will be the last 20

[29:24]
from here divided by 20. The SMA of this

[29:26]
candle would be the last 20 divided by

[29:28]
20 you know and you can find basically a

[29:31]
time series SMA over time. Um that is

[29:34]
not something we forward through the LM.

[29:35]
uh if you look at

[29:38]
uh the project that we're following,

[29:40]
they don't forward the SMA, they forward

[29:42]
the EMA. They forward the mid prices,

[29:45]
that's the last 10 prices of Bitcoin and

[29:48]
then EMA on a 20 period, not SMA, EMA.

[29:51]
So there's a subtle difference between

[29:52]
not a subtle difference of mild

[29:54]
difference or a big difference between

[29:55]
what EMA and SMA are. SMA hopefully is

[29:57]
easy to digest. The SMA 20 for this

[29:59]
candle would be the last 20 candles mid

[30:02]
price divided by 20. EMA or exponential

[30:05]
moving average is a little different. Um

[30:08]
EMA takes into consideration the recent

[30:11]
candle a little bit more than the older

[30:13]
candles. So if there if you're

[30:14]
calculating EMA 20 for this specific

[30:17]
time, you will take the last 20 candles.

[30:19]
But for this candle, the effect it will

[30:23]
have on EMA 20 won't be as much as this

[30:25]
final candle. So the final candle as you

[30:27]
keep moving towards the right um the

[30:29]
candles on the right will have a little

[30:30]
more effect on EMA um or the exponential

[30:33]
moving average compared to the candles

[30:34]
on the left. That's the difference

[30:35]
between an SMA and an EMA. Um

[30:37]
technically it follows this formula and

[30:40]
if you sort of do a quick experiment for

[30:42]
example I did this experiment um before

[30:45]
starting the video. If this is what the

[30:48]
chart looks like um let's say the candle

[30:50]
was the mid price of the candle was 2011

[30:52]
then 208 215 215 probably 225 and then

[30:56]
250 I've written them down over here so

[30:58]
if the mid prices look like this there

[31:01]
is clearly an upward trend over here um

[31:03]
the SMA comes down to 222.60 60. So

[31:07]
somewhere over here, this is where the

[31:09]
SMA comes out. Um the EMA is slightly

[31:12]
above it because you can clearly see the

[31:15]
recent candles are on a up upward trend.

[31:18]
If the recent candles were on a downturn

[31:20]
trend, then the EMA might be lower than

[31:22]
the SMA as well. Um so the EMA takes

[31:25]
into considerations the recent candles a

[31:27]
little bit more. The recent candles are

[31:28]
a little more weighted compared to the

[31:30]
older candles. um which is why in this

[31:32]
case since the recent candles are going

[31:34]
up the EMA is above the SMA but if the

[31:36]
recent candles were going down the EMA

[31:38]
would have been lower to the SMA. This

[31:40]
is a common sort of financial indicator

[31:43]
that is used which doesn't do much up

[31:46]
like this isn't the only indicator that

[31:48]
will tell you anything. Um there are

[31:49]
some indicators derived on top of EMA um

[31:52]
which usually give you some more

[31:53]
information. Um that is the next one

[31:55]
over here which is moving average

[31:57]
convergence divergence. So this is the

[31:59]
fourth one. To recap, let's go to the

[32:02]
simpler ones first. Mid prices, just the

[32:03]
last 20, last 100, last 200 prices. Um

[32:06]
or mid prices, which means in a specific

[32:08]
interval, what was the average price?

[32:10]
SMA or simple moving average, that's you

[32:12]
know the moving average of the last 20

[32:14]
candles, last 30 candles. Um in a fairly

[32:16]
simple mean sort of a formula, the sum

[32:18]
of all the mid prices divided by the

[32:20]
total number of intervals. Um EMA, which

[32:23]
can be of various types. EMA 20 which

[32:25]
means the last 20 candles, EMA 50 means

[32:27]
the last 50 candles. Um is the

[32:29]
exponential moving average which is

[32:31]
calculated using this formula. If you

[32:32]
read through this formula um before

[32:34]
writing this formula you have to

[32:35]
understand what a multiplier is or you

[32:37]
know you first calculate the multiplier.

[32:39]
Let's say you're calculating EMA 20. In

[32:41]
this case the multiplier would be 2 /

[32:43]
21. 2 divided by the number of periods

[32:46]
that you're calculating it for plus one.

[32:48]
So that will be whatever 2I 21 is like 1

[32:51]
or 098

[32:54]
something like that a small number

[32:55]
basically um and EMA today. So if you're

[32:58]
calculating the EMA uh or the moving

[33:00]
average the EMA at a certain candle

[33:02]
would be the EMA at the last candle into

[33:05]
one minus multiplier plus the price

[33:08]
today into the multiplier. So you sort

[33:10]
of calculate the moving average and the

[33:12]
current moving average of a candle

[33:13]
depends on the moving average of the

[33:15]
last candle. The EMA today, the EMA at

[33:18]
this point is going to be the EMA up

[33:21]
until this point into 1 minus

[33:23]
multiplier. EMA yesterday into 1 minus

[33:25]
multiplier plus EMA plus the price at

[33:29]
this candle which will be this mid price

[33:31]
right here into the multiplier. This

[33:33]
calculates the EMA for the day. Um, one

[33:36]
good question you might have is what is

[33:38]
the very first EMA? Um, if the EMA over

[33:41]
here depends on this guy and the EMA

[33:42]
over here depends on this guy, how do

[33:44]
you calculate the EMA of the very first

[33:45]
candle? Which is a great question. Um, I

[33:47]
think there are various ways. I could be

[33:48]
wrong on this, so check yourself. But

[33:50]
first, you calculate the SMA of let's

[33:52]
say these 20 candles. I repeat, you

[33:54]
calculate the SMA of these 20 candles

[33:56]
for this period. And then the very first

[33:58]
EMA is the SMA of the last 20 candles.

[34:00]
So if you are let's say calculating EMA

[34:02]
20 for these last 10 candles, you will

[34:05]
first find the SMA of these 20 candles.

[34:07]
I repeat if you are calculating EMA 20

[34:10]
of these 10 candles if you want I want

[34:12]
EMA 20 at this point this point this

[34:14]
point this point so on and so forth what

[34:16]
you'll do is you'll calculate the SMA

[34:17]
for the last 20 candles and that will be

[34:19]
the very first EMA and then this EMA

[34:21]
will depend on this EMA this EMA will

[34:23]
depend on this EMA this EMA will depend

[34:24]
on this EMA so on and so forth so first

[34:26]
you the very first EMA is considered to

[34:28]
be the SMA and then every subsequent EMA

[34:31]
just depends on the EMA before it um is

[34:33]
that clear do we understand mid prices

[34:35]
SMA and EMA

[34:37]
The last one which again they do forward

[34:39]
which is why why we should probably also

[34:40]
forward it which is MACD. Um, this one

[34:43]
right Oh, see this one right here.

[34:47]
MACD stands for moving average

[34:49]
convergence divergence which is a like a

[34:53]
lot to say. Um, it means a lot of

[34:56]
things. Um, using this indicator you can

[34:58]
calculate all things. Technically, it's

[35:00]
a very simple thing. It's the EMA 26

[35:02]
minus the EMA 14. Um, it is the 26th. So

[35:06]
if you calculate u EMA considering the

[35:10]
period to be 26 minus if you calculate

[35:13]
the EMA consider the considering the

[35:15]
period to be 14 or actually this could

[35:16]
be any two numbers. This is the popular

[35:17]
one EMA 26 minus EMA 14 but this could

[35:20]
be any two sort of periods that you

[35:22]
select. Um this is called the moving

[35:24]
average convergence divergence. Um the

[35:27]
reason it's called a moving average

[35:28]
convergence divergence is because it

[35:30]
shows you how the moving average a

[35:32]
short-term and a long-term moving

[35:33]
average is converging or is it

[35:35]
diverging? Are they moving towards each

[35:36]
other or are they moving away from each

[35:37]
other and based on this information um

[35:39]
you can you know understand something

[35:41]
about the market if they start to

[35:43]
converge or if they ever overlap is when

[35:44]
you understand the market might be going

[35:46]
in one direction or the other and if

[35:48]
they're diverging too much is when also

[35:50]
you can you know make a prediction the

[35:51]
market will go in a different direction.

[35:53]
These are the three indicators we'll be

[35:55]
forwarding. We'll not forward SMA. It

[35:57]
was just the reason we understood SMA is

[35:59]
because EMA depends on SMA. um mid

[36:02]
prices, EMA and MACD are the three

[36:05]
things we need to forward to the LLM. Um

[36:07]
the LM will then respond with the

[36:09]
positions that it has to take and we'll

[36:10]
go from there. For that, we will need

[36:12]
the last let's say 50 candles um from

[36:15]
the LLM. The LM will tell us the 50

[36:17]
candles. Based on those 50 candles, we

[36:19]
will calculate the mid prices.

[36:24]
Cancel.

[36:27]
We will calculate the mid prices. We'll

[36:28]
calculate the EMA. calculate the MACD

[36:31]
and that's all we will need uh to

[36:32]
forward to the L&M. Hopefully that is

[36:35]
clear. I think in the next section let's

[36:36]
code these up. Um we should first

[36:39]
probably code um yeah these indicators.

[36:42]
Um so we'll write the logic to get the

[36:44]
last 20 candles from the LM um or last

[36:47]
50 candles from the LM and calculate

[36:48]
these numbers and maybe print them on

[36:50]
the screen. We'll go from there and you

[36:52]
know see how we can connect the uh LM to

[36:54]
it on one side the exchange to it on the

[36:55]
other side. Um [snorts]

[36:58]
if you look at uh the data for from

[37:02]
these guys there are two broad set of uh

[37:05]
indicators that they send. There is the

[37:07]
intraday series and then there is the

[37:09]
long-term context um which basically

[37:11]
means they send these indicators for the

[37:13]
last 20 candles um considering those

[37:15]
candles to be 1 minute or 3 minute

[37:18]
candles. So like a shorter trend of the

[37:20]
last 1 hour and then also a much longer

[37:23]
trend of the last 20 hours or you know

[37:26]
50 hours is also what they send. So they

[37:28]
send this

[37:31]
all the midprices SMA MCD or sorry EMA

[37:34]
MCD and for a smaller interval also the

[37:37]
last 20 minutes they send the same

[37:39]
thing. this long-term and short-term

[37:40]
sort of context um tells the LM you know

[37:43]
or helps the LM make the final decision

[37:45]
on uh whether it wants to u buy or sell

[37:48]
or long or short okay so we'll basically

[37:50]
write this up we'll write both of these

[37:53]
uh sort of indicators everything over

[37:56]
here and then everything over here um

[37:58]
and sort of print them on the screen for

[38:00]
let's say one market just the Solano

[38:01]
market and then we can extrapolate that

[38:03]
logic to BTC market Ethereum market uh

[38:05]
whatever other sort of assets that they

[38:07]
support um we'll see how many of these

[38:09]
assets we support will probably support

[38:10]
the same so you know they support I

[38:11]
think six assets we'll also support the

[38:12]
same six assets um and go from there

[38:14]
moving on to the next section

[38:19]
all righty let's kick things off I have

[38:22]
signed up on lighter the link is

[38:25]
applierxyz

[38:27]
as soon as you go over here this is a

[38:29]
decentralized per platform which means

[38:32]
you will have to sign up using a

[38:34]
decentralized wallet they might have

[38:36]
login with Google as well actually let

[38:38]
me quickly confirm on that. Um, but I

[38:40]
would assume most probably they don't.

[38:42]
Um, in which which basically means

[38:44]
you'll need some sort of Yeah, they do

[38:45]
have login with email. So, you can

[38:46]
probably just log in with email in case

[38:49]
wallets seem a little foreign to you,

[38:50]
but generally I suggest um, you know,

[38:53]
getting yourself an Ethereum wallet

[38:54]
which is free um, and signing up using

[38:57]
that. Um, once you do, you

[38:59]
[clears throat] will have to on ramp

[39:00]
some money in here. So, you'll have to

[39:02]
send or deposit some money. The way to

[39:03]
do that is click on the deposit button.

[39:06]
And here is where things get tricky. U

[39:08]
unless you have funds in another crypto

[39:12]
wallet or a crypto exchange, uh it'll

[39:14]
get tricky for you to put funds in here.

[39:16]
For me, I was able to on ramp funds

[39:18]
here. So, I have 100 USDC currently to

[39:20]
trade with. Um

[39:23]
now, I can create an account. So, they

[39:24]
have some sort of restriction. Only if

[39:26]
you sign up and deposit some USDC uh at

[39:29]
least 5 USDC can you create an account.

[39:30]
I have created an account over here. If

[39:32]
all of this feels out of your league

[39:34]
because you've not dealt with

[39:36]
decentralized wallets, if you don't know

[39:37]
what USDC is, um then probably, you

[39:40]
know, do the same on a centralized

[39:41]
exchange like coin DCX or Zerx, U

[39:44]
Binance, whatever. Um but I'm going to

[39:47]
use this per platform to trade today or

[39:50]
whatever, you know, write the the logic.

[39:52]
Um this is what it looks like. Um this

[39:54]
is the screen where we'll have all the

[39:56]
tickers. Um specifically a few markets

[39:59]
that we'll probably choose would be the

[40:00]
top six I would say BTC hype soul BNB.

[40:04]
Um if you look at the soul chart right

[40:06]
now this is what the candles look like.

[40:07]
Um but we want all of this data in u

[40:11]
programmatic format on a script. Um so

[40:13]
what I'm going to do now is initialize a

[40:15]
fresh project. Um

[40:18]
second let me move my terminal here. um

[40:22]
a fresh project called

[40:26]
AI trading or something like that.

[40:35]
I'm going to open this in cursor which

[40:37]
is the

[40:39]
IDE of my choice

[40:43]
and I'm going to move cursor over here.

[40:52]
I'll create a fresh bunj project. So bun

[40:56]
init.

[40:58]
Yeah, I think this is fine. Followed by

[41:00]
a bun add a few dependencies. Um the

[41:04]
dependencies here are going to well

[41:06]
there'll be a bunch of them. Let's kick

[41:08]
things off with just the lighter

[41:10]
JavaScript SDK u that I wrote a while

[41:13]
back. I have not released this on npm so

[41:15]
I I'll have to clone this. Um,

[41:18]
but feel free to eventually uh, you

[41:22]
know,

[41:24]
publish this to npm and then just do a

[41:26]
bun ad.

[41:31]
This is probably not the most optimal

[41:32]
way to uh, to do what I'm doing. All

[41:35]
right, we have the lighter

[41:38]
SDK u, lighter typescript SDK as a

[41:42]
subdirectory over here, which means in

[41:44]
index.ts TS icon import functions from

[41:46]
the SDK. Um, specifically to kick things

[41:48]
off, we need the candlestick data. And

[41:51]
I've also added a few examples here. I

[41:53]
think I have the candlestick data in

[41:54]
here. It's this one right here. So, you

[41:56]
can use this as an example. Um, to

[41:59]
understand how do you get um the

[42:01]
candlestick data for a specific market.

[42:03]
If you look at it, it's fairly simple.

[42:04]
Um, you import specifically just this

[42:08]
candlestick API which is a wrapper

[42:10]
around a bunch of HTTP calls. Um this

[42:12]
wrapper would let you hit a specific

[42:14]
market. So you can for example hit the

[42:16]
soul market which has an ID of two or a

[42:19]
BTC market which I think has an ID of

[42:20]
zero and then ETH market which has an ID

[42:22]
of one so on and so forth. Um you

[42:24]
[snorts] can tell it what u time series

[42:26]
data do you need. Do you need the 1

[42:28]
minute candles? Do you need the 5m

[42:30]
minute candles? You need the 3minut

[42:31]
candles. You need the 4hour candles. For

[42:33]
us I think we'll I think what arena the

[42:36]
arena that we saw they do a 3minut

[42:38]
candle and they do a 4hour candle. So

[42:39]
these are the two that we'll try to

[42:41]
fetch programmatically. Um

[42:44]
what time do you want it from? So let's

[42:46]
say you want it from the last in case of

[42:49]
a 1 minute candle. If you need the last

[42:51]
50 candles, we need the last 50 minutes

[42:52]
of data. This would be the current date

[42:55]
minus 50 minutes. This would be the

[42:56]
current date. And you can also provide

[42:58]
how many candles do you need. In our

[43:00]
case, we'll need the last 50 candles. So

[43:01]
we can pick these offsets to be a little

[43:03]
bit more. Um let's [snorts] say the last

[43:05]
I don't know 2 hours. So the current

[43:08]
date minus 2 hours to the current date

[43:09]
and only the last 50 candles is what we

[43:11]
need. That's all the data that we're

[43:12]
going to forward to the LM. We don't

[43:14]
really need 100 candles or 200 candles

[43:15]
or 500 candles. Um so we can copy this

[43:18]
code or you can write it yourself. Um

[43:23]
and let's say we're uh let's replace

[43:27]
this to take the market ID as an

[43:30]
argument. Let me turn on cursor. Sorry,

[43:33]
turn off cursor. Let me pass the sole

[43:36]
market ID for now. Um let's see if it is

[43:39]
able to return us the last 50 candles.

[43:43]
Um given this is 24 hours of data. We

[43:45]
don't need that. Just in the last 1 hour

[43:48]
um the latest 50 candles for the 3

[43:51]
minute data. And let's log that and see

[43:54]
if we get something useful. Um the only

[43:56]
thing is this will be dot dot will be

[43:58]
dot / lighter typescript [snorts]

[44:01]
SDK/generated. Um, let's do a

[44:05]
bun index.ts.

[44:09]
Uh, let's do a bun install

[44:12]
followed by a bun index.ts

[44:18]
cd lighter SDK.

[44:22]
This is why I should have packaged

[44:26]
and published the thing on npm. I have

[44:28]
not done that. This is why we have this

[44:29]
weird folder inside a folder. Um, I

[44:31]
think this should still work. Bun

[44:33]
index.ts returns us some bad data. It

[44:36]
says 3 minute is not allowed. 1 minute,

[44:40]
5 minute, 15 minutes. H. All right,

[44:42]
let's go with 5 minute data then. Um,

[44:48]
and if I look at five minute data, I do

[44:50]
get back a bunch of candlesticks.

[44:52]
Hopefully, this is the latest 50

[44:53]
candlesticks. How can we confirm? If you

[44:55]
look at the last candlestick, it's at

[44:57]
open 198.626.

[45:00]
and close 198.929. So I can pretty much

[45:04]
look at the latest candle here 626 that

[45:07]
looks right and then close was around

[45:09]
929 which looks right. The one below it

[45:12]
before it started around 198.845

[45:16]
ended around 198.626

[45:18]
um

[45:20]
626 looks right and 190.08

[45:24]
uh 198.08 08 which looks wrong to me but

[45:28]
that's because this is 3minut data and

[45:30]
we were looking at 5m minute candles. So

[45:31]
let's look at the 5minute candles and

[45:33]
now this looks right. 199.0 to 199 619.0

[45:39]
to 199.081

[45:42]
is what we have here

[45:45]
which looks wrong to me. Um high and

[45:48]
then sorry close 626

[45:50]
and do we have 626 here? We do have

[45:52]
closer to 626 over here. Perfect. Um,

[45:56]
that looks perfect to me. I think this

[45:57]
is fine. Let's look at one more candle

[45:59]
before it. The open should be 199.026.

[46:03]
Close should be 199.072.

[46:07]
Uh, close 072.

[46:10]
Open 199.03.

[46:13]
199.0.

[46:16]
Let's see.

[46:18]
Open is 199 point right here. Looks like

[46:22]
08

[46:24]
0. Yeah, it looks like 199.08

[46:29]
03 would be here. Oh yeah, that is the

[46:32]
open 19.03 and closes at 199.08

[46:37]
03 and closes at 199.08. That's perfect.

[46:41]
Um, so we do have the right candlestick

[46:44]
data coming back. We have the latest 50

[46:46]
candles. The very first thing if you

[46:48]
remember on

[46:50]
again I keep uh forgetting the name of

[46:52]
the project Alfarena um the very first

[46:55]
thing that we would send to the model

[46:56]
would be the mid prices um this thing

[46:59]
right here. So let's calculate the mid

[47:02]
prices for the last 10 candles um as the

[47:05]
first thing. So

[47:07]
after you get the K lines const mid

[47:10]
prices would be um the last 10 candles.

[47:13]
So k lines sorry k lines dot how do you

[47:18]
get the last 50 splice 0 comma

[47:22]
- 10 I think or slice 0 - 10 should

[47:25]
probably give me the last

[47:28]
10 candles um k lines dot candlesticks

[47:37]
dot slice

[47:40]
last 10 candles dot map Uh

[47:44]
so open and close

[47:48]
two open + close divide by two.

[47:53]
Let's log it and see if the mid prices

[47:56]
look fine.

[47:59]
The mid prices look something like this

[48:02]
which doesn't look too wrong to me. They

[48:04]
look like there are more than 10 here.

[48:06]
Um which seems wrong. I think slice is

[48:09]
incorrect. Then get last x

[48:14]
elements JavaScript

[48:18]
uh

[48:21]
dot slice minus number of elements. So

[48:23]
something like this. Let's try that.

[48:26]
Perfect. 1 2 3 4 5 around the last 10

[48:28]
candles. Probably only need

[48:32]
uh it until two uh decimal points. So

[48:36]
bot two fixed should probably do that.

[48:39]
That looks fine. Um there are strings

[48:41]
and not numbers, but again I think it

[48:43]
doesn't matter um for the LLM. So that's

[48:46]
great. We're able to get the mid prices.

[48:47]
What is the other thing we need? We need

[48:50]
the

[48:55]
EMA 20. Um the question is how do we

[48:58]
calculate the EMA 20? If you remember

[49:00]
the formula it looked something like

[49:02]
this. The EMA 20 is price today into

[49:07]
multiplier plus price yesterday into or

[49:10]
sorry EMI EMA EMI yesterday into one

[49:12]
minus multiplier. So let's just create a

[49:14]
function um probably in a separate file

[49:16]
called um indicators.ts export function

[49:21]
uh get EMA. Let's take the period as an

[49:25]
input. So you know period basically

[49:27]
represents

[49:31]
presents or you know at pari

[49:40]
the EMA period

[49:44]
period for which the EMA is being

[49:47]
calculated.

[49:50]
So if you're calculating EMA 20, the

[49:51]
input would be 20. Uh the modifier or

[49:54]
the multiplier

[49:57]
would be 2 divided by period

[50:01]
+ one. Um we also need the candlesticks

[50:05]
or the prices I would say should be a

[50:07]
number array. Um and given these prices,

[50:12]
what we want is um

[50:15]
if the array looks something like this.

[50:18]
let's say 1 2 3 4 5 and you're

[50:20]
calculating EMA

[50:23]
3 let's say um then we'll first take the

[50:26]
simple moving average of these two. So

[50:27]
that's 1 + 2 divided by 3 uh 1 + 2 divid

[50:30]
by 2 that's 1.5 and then we'll start to

[50:32]
calculate the EMA 1 EMA 2 EMA 3. So

[50:36]
we'll first take the

[50:38]
prices.length minus period prices.length

[50:41]
length five minus period 3. 5 - 3 is 2.

[50:45]
We'll take the first two elements. We'll

[50:47]
find the SMA amongst these two. And then

[50:50]
we will apply the formula that's present

[50:52]
over here which is EMA today is equal to

[50:55]
price to multiplier plus EMA yesterday

[50:57]
into 1 minus multiplier. Basically the

[50:59]
question is how do you get the very

[51:00]
first EMA? Um because EMA today depends

[51:03]
on EMA yesterday. So at some point you

[51:05]
need the very first EMA which is just

[51:07]
the SMA of all the candles that come

[51:09]
before.

[51:10]
>> [clears throat]

[51:10]
>> So what can we do over here? We'll first

[51:12]
say const um SMA interval equal to

[51:16]
prices.length

[51:17]
minus uh period.

[51:20]
If SMA interval is less than one um then

[51:25]
return or you know probably throw an

[51:28]
error that says

[51:30]
not enough

[51:32]
candles provided

[51:36]
or let's just call them prices. But if

[51:39]
we do get enough data is when we're

[51:42]
going to first calculate the SMA. So

[51:43]
const SMA equal to Z or you let SMA

[51:47]
equal to Z.

[51:49]
Um for

[51:52]
let I equal to zero I less than uh

[51:57]
SMA interval I ++

[52:00]
SMA plus equal to prices of I and then

[52:04]
SMA divide by equal to um

[52:08]
SMA interval.

[52:11]
What's the problem here?

[52:14]
prices of I would exist.

[52:17]
So let's just make it zero if it doesn't

[52:20]
exist. But it would definitely exist

[52:22]
over here because I is less than SM

[52:24]
interval which this thing will sort of

[52:26]
check. Okay, we have enough

[52:28]
elements in the array before that point.

[52:30]
So even though TypeScript is complaining

[52:31]
over here, it ideally shouldn't. If you

[52:32]
use a for each loop, it wouldn't. Uh but

[52:35]
let's not go there. We have the SMA at

[52:37]
this point. Now we can create a new EMA

[52:41]
array. the very first element of which

[52:42]
will be this SMA. This SMA is the the

[52:45]
simple moving average is the very first

[52:46]
element and then we can calculate the

[52:49]
next 20 you know um

[52:51]
EMAs from it. Um so for let I equal to0

[52:56]
I less than U period I ++ now we'll

[53:00]
start to calculate all the EMAs from

[53:02]
here. Um so con EMA equal to EMAs of the

[53:08]
last EMA. So, EMS of EMA is dotlength

[53:12]
minus one um

[53:16]
into multiplier

[53:20]
into 1 minus multiplier. My bad.

[53:24]
Plus the price today. So, that's going

[53:26]
to be oh boy. Prices of

[53:31]
SMA interval plus I. Um so we're going

[53:34]
to take everything the first n elements

[53:36]
we have taken for the SMA um for

[53:38]
calculating the SMA. Now we need the

[53:40]
first n elements plus 0 + 1 + 2 like

[53:42]
that. Um which is why it might be an i

[53:45]
or it might be an i + 1. I'll log and

[53:47]
see eventually which one. Um but one of

[53:49]
these we need over here into the

[53:52]
multiplier.

[53:54]
And again this should exist u typescript

[53:57]
is complaining but let's just u make

[54:00]
typescript not complain here as well as

[54:02]
here.

[54:04]
and then uh

[54:09]
emasp

[54:13]
console.log. So let's just see I mean

[54:15]
I've written this fairly ad hoc um but I

[54:17]
think it should be fine. Um

[54:20]
don't need that. The get ema function

[54:23]
takes the prices array as an input. So

[54:26]
that looks something like this. The last

[54:28]
50 candles in our case. So the last 50

[54:30]
mid prices um and [clears throat]

[54:35]
we will first find the SMA until this

[54:38]
point for the first prices.length minus

[54:42]
period candle. So the prices do.length

[54:43]
is 50 and period is 20. For the first 30

[54:46]
candles we calculate the SMA that is

[54:48]
what we consider to be the very first

[54:49]
starting EMA. The EMA gets initialized

[54:52]
with the SMA of a bunch of candles that

[54:54]
come before the final period element. If

[54:57]
the period is 20, if we're calculating

[54:59]
the EMA, 20 20 elements, whatever

[55:02]
elements exist before the last 20

[55:04]
intervals is what we use to calculate

[55:07]
the SMA. That's the very first EMA, then

[55:09]
hopefully the formula that I've written

[55:10]
is correct over here. Um, and that will

[55:12]
just help us calculate all the in in

[55:14]
case of EMA 20, the last 20 EMAs. Um,

[55:17]
and then we'll just log it and also

[55:19]
return it.

[55:23]
So this should return

[55:26]
a array of numbers.

[55:30]
And if I go back here, let me also

[55:33]
create a function called

[55:35]
con

[55:38]
export function get mid prices. So this

[55:41]
should get mid prices giving a bunch

[55:43]
given a bunch of candlesticks. Um that

[55:47]
would be so candlesticks.

[55:52]
Candlestick

[55:54]
I saw type in the logs. I think it is

[55:56]
this is the right type. That looks right

[55:57]
to me. Um

[56:00]
array and this should basically return

[56:02]
what we wrote in index.ts which is this

[56:06]
map

[56:08]
minus the slice

[56:11]
dot map.

[56:13]
This looks wrong because candlesticks

[56:17]
and you rather than doing this I'll

[56:19]
simply say con mid price is equal to u

[56:24]
get mid prices of k lines dot

[56:27]
candlesticks uh that's it here if you

[56:31]
want we can just get just the last 10 um

[56:34]
or we can just log

[56:37]
midprices doss slice

[56:39]
minus 10 if you just want to get the

[56:42]
last 10 elements

[56:49]
uh but

[56:51]
but let's just create a generic function

[56:52]
that takes all the candlesticks and

[56:54]
returns all the mid prices. So now we

[56:56]
have all the the latest 10 mid prices.

[56:59]
We also have const EMA is equal to get

[57:02]
EMA um the prices are going to be all

[57:05]
the mid prices and the period is going

[57:07]
to be let's say 20. So let's just call

[57:09]
it EMA 20s console.log log EMA 20s and

[57:14]
here we get an error string array is not

[57:16]
see I should probably

[57:18]
convert this to a number

[57:21]
dot two converts the thing to a string

[57:23]
so I just convert that back to a number

[57:25]
um and that should do it hopefully we

[57:27]
now are able to log not just the mid

[57:28]
prices but also the EMA 20s the last 20

[57:32]
EMA 20s uh but again I think alpha arena

[57:37]
alpha arena only returns the last 10 EMA

[57:40]
20s um as you can see over here. So,

[57:45]
that is what we're going to sort of log.

[57:46]
So, we'll only log the last 10 EMA 20s.

[57:50]
We're able to log the mid prices, log

[57:52]
the EMA 20s. Um, I'm unsure if this is

[57:55]
correct, but let's see. Um,

[57:58]
this is the mid prices. This is the EMA

[58:02]
20s. If you look at the trend, um, it's

[58:06]
gone up, it's gone down, it's gone down,

[58:10]
it's gone down. Um, and this seems to

[58:13]
follow and then it's gone up. I should

[58:16]
write tests for this. I'm unsure if the

[58:17]
form if what the code that I've written

[58:18]
is correct or not. Um, so I should

[58:20]
definitely write tests for get EMA. Get

[58:23]
some real data and just make sure u that

[58:26]
this function does what it's supposed to

[58:27]
do. Um, EMA is done, mid price is done.

[58:30]
The only the third u indicator that's

[58:33]
left is export function get me um which

[58:37]
will

[58:39]
take what as an input it'll take

[58:41]
candlesticks as an input. So prices

[58:44]
which is a number array um and have they

[58:47]
written what intervals they're taking

[58:49]
for MACD? No. So I'm assuming most

[58:51]
probably this is uh MACD with uh EMA

[58:58]
26 minus EMA 14 I think what is the

[59:03]
common one I think it is this only yeah

[59:06]
[snorts] EMA 26 minus EMA 14. So the job

[59:08]
of this guy is to find EMA find EMA 26

[59:12]
find EMA 20 find the difference between

[59:14]
them and that is what is considered to

[59:16]
be the moving average convergence

[59:18]
divergence. So const um EMA

[59:24]
26 equal to get EMA

[59:27]
for these prices with an interval of 26.

[59:30]
EMA 14. We'll get the same thing with

[59:32]
the interval of 14. Now this guy's final

[59:35]
length will be 26.

[59:38]
This guy's final length will be

[59:41]
uh 14. So what we need to do is we need

[59:43]
to basically get the le last 14 candles

[59:46]
from here and then subtract that from

[59:48]
this and then get the last 10 candles or

[59:50]
the last 10 10 MCDs which is what we'll

[59:52]
forward to the LM over here. I think

[59:56]
these are 10 as well. Yeah, these 10

[59:58]
we'll [clears throat] get by first

[60:00]
subtracting first converting

[60:03]
EMA 26

[60:05]
uh dot slice minus 14.

[60:10]
We'll get the last 14 EMA 26s and

[60:14]
we'll

[60:16]
const be equal to dot slice -14 dot map

[60:23]
um

[60:25]
EMA 26

[60:28]
comma index.

[60:32]
Yeah. Um and this should return EMA 20.

[60:36]
I think let's just take the index. EMA

[60:39]
26 of index minus EMA 14 of index.

[60:43]
And again, TypeScript will complain

[60:45]
here. But we know for a fact these

[60:47]
should exist. As long as we're passing

[60:48]
in 50 candles over here, we'll be fine.

[60:51]
Um, so

[60:54]
we can actually just tell Typescript to

[60:56]
ignore these. Um, and we're good to go.

[60:58]
Return

[61:00]
MACD. um const MACD equal to get MACD

[61:05]
with uh the same set of prices. So mid

[61:10]
prices and then console.log MACD dot

[61:13]
slice the last time of these. That was

[61:15]
simpler than I hoped for. I'm also not

[61:16]
sure if this is right. So I I I'll write

[61:18]
some tests just to ensure these

[61:20]
indicators are correct. If they're not,

[61:21]
then we're screwed anyways. Like then

[61:23]
irrespective of how good the model is,

[61:25]
it'll just perform really bad or perform

[61:26]
really good. Um but we should write

[61:28]
tests for these functions. I've written

[61:29]
them fairly ad hoc. I've written a get

[61:32]
mid prices function. This one is

[61:33]
correct. Get EMA and get MECD. We should

[61:36]
confirm. Okay, this is how they're

[61:37]
actually written because I'm unsure. Um

[61:39]
I could totally be wrong on on you know

[61:40]
how I'm how I am interpreting these two

[61:43]
indicators. Let's oopsy. Let's try to

[61:47]
run it.

[61:51]
So we get mid prices, we get uh the EMA

[61:56]
20s and we get the MACDs.

[61:59]
MACD is it supposed to be a number that

[62:02]
looks like this? I don't know. I think

[62:04]
it's between 1 and 100 or something like

[62:06]
that. Um, let's see. Model chat for

[62:09]
Solana. What does it look like here? I

[62:11]
think that should give us a decent idea.

[62:12]
For soul, what does the MACD look like?

[62:16]
Minus.3

[62:18]
minus.3 something like that. For us, it

[62:20]
looks something like this.

[62:22]
That doesn't look great. Let me refresh.

[62:25]
Let me go to model chart. Let me open

[62:27]
the latest one. This is 30358. That's

[62:30]
like 1 minute ago. 1 minute ago for

[62:32]
these guys, what did the soul data look

[62:34]
like? It looked something like this.

[62:35]
199.135. For us, it's starts at 199.25.

[62:39]
Let's just compare them side to side.

[62:41]
And then 199.045,

[62:44]
199.21.

[62:45]
Uh 198.81, ours is 198.88, 198.635,

[62:50]
ours is 1983. So this looks pretty close

[62:53]
to the mid prices here. EMA 19.305, 305.

[62:57]
Ours is at 200 and then the next one is

[63:01]
yeah EMA calculation for us seems wrong.

[63:04]
Um

[63:06]
I could be wrong like it starts from

[63:09]
199.1. It goes up to 199.3 here. Here

[63:12]
the jump is a little too sudden. Uh so

[63:15]
that makes me feel I've not calculated

[63:16]
these correctly. Um

[63:19]
we'll see if that's the case or not

[63:20]
soon. Um it ends at 199.21 21 and it

[63:24]
ends here at 199.7

[63:27]
um 198.7. So little below that. This is

[63:30]
like we're.5 up on all of these

[63:33]
indicators and we're not.5 up on all the

[63:37]
mid prices. Um like 2 up. Um so it may

[63:41]
or may not be correct. I don't know yet.

[63:43]
Um

[63:48]
all right. U seems like I found the

[63:49]
issue. Well, there is no issue. The

[63:51]
problem might be okay they when they

[63:53]
look at the intraday series they look at

[63:55]
3minut intervals and we look at 5m

[63:57]
minute intervals because lighter doesn't

[63:58]
give us 3minut intervals it only gives

[64:00]
us either 1 minute intervals or 5m

[64:01]
minute intervals um it doesn't give us 3

[64:04]
minute intervals which is why the data

[64:05]
is the trend is similar but it's a

[64:07]
little different if you compare the

[64:08]
4hour data though um so if I changed the

[64:13]
candle size to be 4 hours um for the

[64:15]
last you know whatever xyz hours then it

[64:17]
looks very similar if you look at the

[64:18]
MCD or the compare the MCDs Um 1.5 over

[64:21]
here, 1.5 over here, um 1.84, 1.86,

[64:27]
2.16, 2.14,

[64:30]
2

[64:31]
3 4, 2.36, so on and so forth. I've

[64:34]
compared all 10 of them and they look

[64:35]
very close. So our calculation is

[64:37]
correct. The problem is we're taking

[64:39]
5minute candles and these guys are

[64:41]
taking three-minute candles for the

[64:42]
short-term data. Unfortunately, lighter

[64:44]
doesn't provide three-minute data in the

[64:46]
candlestick. So we just have to roll

[64:47]
with it. We can either select the 1

[64:49]
minute candles or the 3minut candles or

[64:51]
the 5minute candles sorry over here. We

[64:53]
cannot unfortunately select 3minut

[64:55]
candles. Um so our data what we're

[64:57]
sending to the LM is unfortunately going

[64:58]
to look a little different compared to

[65:00]
these guys. Logic wise though um let me

[65:03]
just take you through

[65:05]
all three of the indicators one more

[65:06]
time. Um when we calculate the EMA um we

[65:10]
first take the period let's say 20 and

[65:12]
then calculate the SMA using the last 20

[65:16]
candles or the first 20 candles um and

[65:18]
then from there we calculate the actual

[65:22]
EMA. So the SMA is calculated for the

[65:23]
first 20 candles and then we calculate

[65:25]
the EMA. We take the last 10 EMA values

[65:27]
and forward it to the LM. Um for the the

[65:30]
MACD we take the EMA 26. EMA 12 EMA 12

[65:33]
will be longer than EMA 26 because um

[65:36]
we're sending 50 candles over here or 50

[65:39]
uh prices over here. EMA 12 will have 50

[65:42]
- 12 or 50 minus

[65:46]
12 prices. So that's 38. EMA 26 will

[65:50]
have 50 - 26 that's 24 candles. Um so

[65:52]
EMA 12 will actually be longer. The

[65:53]
array of the length of EMA 12 will be

[65:55]
longer than EMA 26. So we just take the

[65:59]
length of EMA 26 which in our case would

[66:00]
be

[66:02]
24 um and this would be 38. We just take

[66:06]
this length 24 and then we subtract the

[66:08]
corresponding values um from EMA 12

[66:11]
minus EMA 26 and that's what we return

[66:13]
over here looks very close at least on

[66:16]
the 4hour candle um to you know the data

[66:18]
over here. What we can do is we can

[66:20]
actually check this for a different

[66:21]
market like BTC. Um I think the BTC ID

[66:25]
is um

[66:28]
zero. I could be wrong. I'm just

[66:30]
assuming here honestly. Um if I do

[66:33]
select the market ID to be zero. This

[66:34]
looks like Ethereum data 3800. I don't

[66:37]
think ETH is at 3,800. Is it? ETH is at

[66:41]
4300. What is at 3800? I don't know. Um

[66:44]
ETH is probably one. Uh so let's just

[66:46]
change this to one and let's see what we

[66:49]
get.

[66:52]
Do we get Ethereum data? No, that looks

[66:53]
like Bitcoin data. So that's perfect.

[66:54]
Um, also I Oh, wow. That's a lot of

[66:58]
candles. Uh, oh, that's because I have

[67:01]
taken the 1 minute interval. Let me take

[67:02]
the 4hour interval. Try that one more

[67:05]
time. So, these are the last 20 prices

[67:08]
um in the 4hour interval. Let's compare

[67:10]
the MACDs. So, if I open this and look

[67:15]
at the Bitcoin long-term data, um

[67:20]
598,

[67:22]
629, 717,

[67:25]
761, um 822, 879, 928, 952, um 1 185,

[67:33]
1071, 1 96, 1287, 1 328,1385,

[67:39]
1 1392, 1 329, and the Last one is there

[67:43]
are more than 10 here. 1 2 3 4 5 6 7 8

[67:46]
9. There are 10 here. What's the last

[67:49]
one again?

[67:51]
1374. And I was just 1 296.

[67:54]
The one before that 139 to1329. Yeah,

[67:57]
close enough. Uh I would probably want

[68:00]
to do this for one more Ethereum, let's

[68:02]
say. Um so let's quickly

[68:05]
try to get the market ID for Ethereum.

[68:08]
Um it's a little weird how you get the

[68:10]
market ID here. I mean the way I have to

[68:12]
do it usually is just refresh and

[68:14]
inspect the requests that go out and

[68:17]
then figure out there you go what the

[68:19]
market it is zero indeed. Um if I select

[68:22]
the market ID to be zero that is for

[68:25]
Ethereum. If I run it um yeah 4165 looks

[68:30]
like the right price. And if you compare

[68:32]
the MECDs

[68:33]
um

[68:36]
then it is for the long-term position

[68:38]
12.4 7 that looks a little far um 21 16

[68:44]
um 29 23

[68:48]
32 34 46 45 56

[68:54]
58 66 62 60 I mean close enough I guess.

[68:58]
Um so that's good. I'm going to assume

[69:00]
that our logic is correct here. It could

[69:03]
of course be wrong. It looks close

[69:05]
enough for the long-term context. It

[69:08]
doesn't look very close for the intraday

[69:10]
data. But I think the reason for that is

[69:12]
um that our candle duration is

[69:14]
different. Um let's quickly log these

[69:16]
for both the long-term and the

[69:17]
short-term position. So console.log

[69:20]
long-term position looks something like

[69:22]
this. Um and then let's do the same for

[69:25]
a short-term position. Let's just

[69:26]
convert this to a function. uh function

[69:30]
get indicators for a specific um

[69:34]
duration which in our case could either

[69:36]
be 1 minute or could be um 4 hours.

[69:40]
Let's just do 5 minutes and 4 hours.

[69:44]
And then we can just paste everything

[69:50]
here.

[69:52]
Let's make this an async function.

[69:55]
Let's change this to duration

[69:58]
and let's change this to position

[70:03]
duration. And then let's just log both

[70:05]
of them. Uh get indicators for 5 minutes

[70:09]
and 4 hours. Um and

[70:13]
probably need to take the market ID as

[70:16]
an input. Probably need to send the

[70:19]
market ID over here. Uh [snorts]

[70:25]
sorry over here

[70:28]
market ID already perfect let's run that

[70:32]
and hopefully we see um oh boy one thing

[70:35]
we should change is this duration um we

[70:38]
get a lot of data for the short-term

[70:41]
position which we don't need um oh boy

[70:43]
it's going to be a little okay uh

[70:46]
duration is 5 minutes then we need not

[70:50]
enough data. Probably 2 hours of data

[70:53]
would do. Um, but if not, then we

[70:55]
probably need 96 hours of data. That

[70:57]
looks right to me. And if I do this now,

[71:01]
that looks right. A long-term position,

[71:04]
a 5 minute position looks something like

[71:06]
this.

[71:08]
And a 4hour position looks something

[71:10]
like this. Um, given

[71:14]
and we should probably make it return um

[71:17]
all of these. Let's not log them

[71:19]
anymore.

[71:21]
return mid prices MACD and uh

[71:26]
EMA 20s which I forgot to delete that.

[71:31]
There you go. And let's just slice them

[71:35]
to just 10 values. Um

[71:37]
both of these and mid prices as well.

[71:41]
All right, that looks good. Um we have a

[71:44]
function called get indicators. I can

[71:46]
get rid of everything else. Let me

[71:49]
export this function. Um

[71:54]
whose job is simple. Um the job of the

[71:56]
get indicators function is to give a

[71:57]
duration and a market ID. Return these

[72:00]
sort of indicators. The mid price, the

[72:02]
MACD and the EMA 20s. Um now comes the

[72:05]
second part which is let me just move

[72:08]
this to a separate file called uh should

[72:11]
probably just be indicators.ts s but let

[72:12]
me call it um

[72:16]
stock data.ts and let me just export

[72:19]
this from here. Index.ts will just have

[72:22]
you know all the orchestration logic

[72:23]
between the exchange on one side and the

[72:25]
LLM on the other side. Next step is

[72:28]
going to be funding our accounts funding

[72:31]
open router probably sending these

[72:34]
writing all the LM log probably next

[72:36]
step. Um so next step what I'm going to

[72:38]
do next is introduce the versal AI SDK

[72:40]
um and start to forward these to the

[72:44]
LLM. Um get a response or a tool call

[72:47]
from the LLM if it wants to long or

[72:49]
short. Um and then we'll in the next

[72:51]
part that comes after that actually

[72:53]
place the order. So let's get into the

[72:54]
next part. Let's do the LLM stuff next.

[72:57]
All right. I actually forgot one thing

[72:58]
before we move on to uh the main part

[73:02]
which is that I also need to get my

[73:06]
current open positions. If I currently

[73:09]
open a small long on Ethereum

[73:13]
then this is my position. The LM is

[73:15]
going to respond back with such

[73:16]
positions and we're going to open those

[73:18]
positions over here. In follow-up

[73:20]
requests, the LLM needs to know okay

[73:22]
this is an open position. this is what

[73:24]
you asked us to create um so that the LM

[73:26]
can perform on top of it. If the LM

[73:28]
wants it might you know might want to

[73:30]
close this position might want to

[73:32]
rebalance this position um to for this

[73:35]
account I also need a way to gather all

[73:37]
the current open positions I think I

[73:40]
also written that in one of the examples

[73:42]
so let's quickly go through it if you

[73:44]
look at all the examples

[73:46]
one of them is get positions.ts ts. Uh,

[73:50]
this actually might not be the best

[73:52]
thing. Um,

[73:54]
but I think it's fine for now. Um, get

[73:57]
positions.ts is can do a call to this

[74:02]
account API. So, that'll probably be on

[74:04]
/ v1/1

[74:07]
accounts. Um, and this will probably

[74:09]
return me my open orders. Let's see if

[74:11]
that's the case or not. Um, but this

[74:14]
would probably require

[74:16]
an API key from my account. Um, yes,

[74:19]
Abita, we've only dealt with open data,

[74:21]
data that doesn't need to be

[74:22]
authenticated. Um, but if you want the

[74:24]
open orders for a specific account, you

[74:26]
need to have an API key for that

[74:27]
account. Um, let me quickly show how to

[74:29]
do that. Go to more uh or tools, click

[74:33]
on API keys, and then add a new API key

[74:37]
over here. Give it an index. I'm going

[74:39]
to give it an index of two.

[74:42]
I'll have to sign this transaction.

[74:45]
That means I'll have to connect my

[74:46]
ledger.

[74:48]
Let me quickly do that.

[75:04]
Another transaction.

[75:10]
And now I get a public key which I might

[75:14]
not need. Um, and a private key which I

[75:16]
definitely need. I will get rid of this

[75:19]
eventually. For now I'm going to just

[75:21]
keep it public so everyone including you

[75:23]
can see it. I'm going to create a new

[75:25]
file called positions or open

[75:27]
positions.ts.

[75:29]
But I'm going to copy this function.

[75:33]
I have the private key and then let me

[75:35]
copy the rest of the things which is the

[75:37]
code. Let me put this private key in a

[75:40]
env file.

[75:46]
Let me import it over API key private

[75:50]
key. So let me just name this to that um

[75:55]
account index. This is the index or the

[75:59]
ID of my account which again there's no

[76:02]
easy way to grab other than refreshing

[76:04]
the page and then finding your ID over

[76:07]
here. There you go. This one is probably

[76:10]
it.

[76:13]
That's my account ID. Um, this is the I

[76:17]
I don't think I need this. This is the

[76:18]
API key index. Um, in lighter you can

[76:22]
create up to 255 API keys. You have to

[76:24]
give each one of these an index. I've

[76:25]
given it an index. I don't think you

[76:26]
need this. The sle market ID. Now, if I

[76:29]
run this file, one

[76:33]
open positions.ts, ts

[76:36]
it gives me an error because the

[76:38]
generated SDK is in dot / lighter SDK ts

[76:42]
/generated if I run it now it returns me

[76:46]
this it says you have a position um

[76:49]
which is of this size it doesn't tell me

[76:53]
if it's a long or a short uh gives me

[76:56]
the unrealized P&L as well gives me the

[77:00]
value of the position um and what price

[77:02]
I entered at value probably means the

[77:04]
quantity that I bought I think no I

[77:06]
bought 0.5 E

[77:09]
that is over here a value means the

[77:11]
current position value of the position

[77:13]
so this number into the price

[77:15]
and what I'm really looking for here is

[77:18]
if it's a long or a short I don't see

[77:21]
that here I see this sign which I

[77:24]
[snorts] think means if it's a long or a

[77:25]
short position so let me close this

[77:26]
position

[77:28]
let me buy a short now

[77:31]
and let's see if this sign changes from

[77:34]
yeah change to minus one. So minus one

[77:36]
means it is a short one means it is a

[77:39]
long. Um let me just change the output

[77:43]
here to be return uh current open orders

[77:48]
data dot accounts dom um

[77:57]
uh symbol

[78:00]
comma

[78:03]
position

[78:05]
comma

[78:07]
sign comma comma

[78:11]
unrealized P&L comma

[78:14]
realized P&L comma

[78:18]
maybe the liquidation price

[78:24]
and just return these in a specific

[78:27]
format. The format is not going to be

[78:29]
this. Uh it's going to be just the sign

[78:33]
is going to be sign is

[78:37]
one then long else short. Um everything

[78:42]
else looks fine to me. What's the

[78:43]
problem here?

[78:46]
Uh symbol doesn't exist on

[78:49]
detailed account. It's account position.

[78:51]
What we got over here? Data accounts of

[78:54]
zero. Oh accounts.

[78:56]
Oh. Oh. Well, this returns the positions

[79:00]
of all of your accounts. I might have

[79:02]
multiple accounts. Um, we will

[79:04]
eventually have multiple accounts, one

[79:05]
for every LLM. Um, so this probably

[79:09]
needs to take the account index as an

[79:11]
input.

[79:13]
Um,

[79:14]
and

[79:18]
market ID.

[79:20]
I think that's fine. Um and we can just

[79:23]
return

[79:26]
the account indexes positions in the

[79:28]
specific format. Um still I see an error

[79:32]
unreachable code detected. Uh

[79:36]
map of this thing. Is there a return

[79:38]
statement before this? No, there isn't.

[79:41]
What's the problem here? symbol symbol

[79:46]
dot

[79:49]
positions is an array of account

[79:50]
position

[79:52]
which is a class which is fine.

[79:56]
All of these are public properties on

[79:58]
that class. So

[80:01]
or you can't dstructure elements from a

[80:03]
class like this can you? Um, so let's

[80:06]
just replace this with account position

[80:09]
and then replace these with

[80:12]
these. I still see a bunch of red

[80:15]
squiggies, but this time I'm hoping

[80:17]
they're solvable. Unreachable code

[80:18]
detected again. Oh,

[80:21]
there we go.

[80:24]
That looks fine to me. Um,

[80:28]
account. Yeah, whoever calls this needs

[80:30]
to call this with the right account

[80:31]
index. Eventually, we'll have multiple

[80:33]
accounts. Um, yeah, let's rename this to

[80:36]
get open positions.

[80:39]
And let's just call this one with once

[80:42]
with the zerooth account.

[80:45]
This returns the positions of the zeroth

[80:47]
account. Let me quickly try to create a

[80:48]
bunch of other accounts. If I go to sub

[80:51]
accounts, I only have one account right

[80:53]
now. Let me quickly create another sub

[80:55]
account called Quen, which might be one

[80:58]
of the models that we deal with

[80:59]
eventually. probably will have to sign a

[81:01]
transaction again. Oh, I don't have to.

[81:03]
Let me create another sub account called

[81:06]
uh

[81:08]
DeepS.

[81:11]
This is where Deepseek would be trading.

[81:12]
It'll have its own capital. Um and you

[81:14]
know, if one gets liquidated, the other

[81:16]
will still sort of remain. Um let me

[81:18]
create another one. What is the other

[81:20]
good one over here? I'll probably just

[81:21]
deal with three LMS by the end and

[81:23]
Claude. So, let's name the third one to

[81:26]
be Claude.

[81:28]
Um, and then let me quickly see if I can

[81:31]
get the positions for the

[81:34]
first account as well, which should be

[81:36]
empty right now because I just created

[81:38]
Oh,

[81:40]
hold on.

[81:44]
Hm.

[81:47]
Why do I get account index the same? Oh,

[81:50]
because I'm logging this. I'm not

[81:51]
logging the other thing. Let me do that.

[81:54]
Undefined. Two open positions undefined.

[81:58]
three open positions undefined. Let me

[82:01]
quickly create a position on the second

[82:04]
sub account. Um, let me transfer

[82:08]
like 10 or let's say $50 US to Deepseek.

[82:14]
Let me sign the transaction.

[82:29]
And now hopefully um my DC account has

[82:33]
$50. My main account has $50. Uh let me

[82:36]
create. Okay, how do I change my sub

[82:38]
account now? Oh, switch. There you go.

[82:42]
Oh god. Everything requires signatures.

[82:57]
All right. [snorts]

[82:58]
Now this is my active account. Yeah. If

[83:02]
I trade on this create a very small

[83:04]
position to let's say long or short, it

[83:06]
doesn't really matter.

[83:09]
Now I have this open position and I had

[83:12]
an open position in my main account as

[83:13]
well. Let's run this um and just log all

[83:17]
the accounts and hopefully we get back

[83:19]
two accounts the deepseek account and

[83:21]
the original account. So there is one

[83:24]
detailed account dot positions.

[83:30]
This is still the Oh, I probably need to

[83:33]
create a different API key for this one.

[83:38]
uh API keys.

[83:41]
Yeah. So we don't need the account

[83:43]
index. We can just assume the zeroth

[83:45]
account is what has the data. Um and for

[83:49]
different API keys um you know different

[83:52]
positions will be fetched. So I was

[83:54]
incorrect in thinking okay this accounts

[83:57]
of X is going to be the sub account. Um

[83:59]
it is not. That also feels weird. Every

[84:01]
sub account needs to have its own set of

[84:02]
API keys. One sub account should be able

[84:04]
shouldn't be able to affect another sub

[84:05]
account. Um, so that makes sense. It

[84:07]
should still be zero. It should not

[84:08]
accept any arguments. We are exporting

[84:11]
this function and we should be good to

[84:13]
go over here. Um,

[84:16]
this is also not used anywhere, which is

[84:18]
surprising. Um,

[84:20]
I'm hoping this is used. This is used

[84:22]
indeed. Yeah. All right, that's good

[84:25]
enough. Um, actually

[84:29]
should probably take the API key as an

[84:30]
input.

[84:32]
uh because

[84:35]
um we'll eventually need this for all

[84:39]
a single process will call this function

[84:41]
again and again with different API keys.

[84:43]
One for the quen account, one for the

[84:44]
cloud account. Um so we should take an

[84:47]
the API key as an input. The account

[84:49]
index would still remain the same. So I

[84:51]
think this is fine. Base URL would

[84:53]
remain the same. So that is fine as

[84:54]
well. Um yeah, we're good to go. Let me

[84:57]
just add one more thing here. Uh account

[85:00]
index

[85:03]
or this thing

[85:06]
and we should be good to go. Um, cool.

[85:10]
Now that that is done, I think now we

[85:12]
have everything. We have a function to

[85:14]
get the open positions um in a slightly

[85:16]
more formatted fashion. Uh and we have a

[85:21]
function

[85:23]
to get back wait for it

[85:27]
to get back the open position is a

[85:28]
function to get back the stock data

[85:30]
which is this one right here um or the

[85:33]
indicators. Now um given we have these

[85:36]
two function we can tell the LM and ask

[85:38]
the LLM to do a bunch of things or trade

[85:39]
on top of the these indicators and my

[85:41]
open positions. Let's get into that

[85:43]
next. Let's now finally get into the LM

[85:45]
stuff.

[85:52]
All right. Um DLM part should be easy.

[85:55]
I'm going to use the OpenAI SDK. Uh

[85:58]
sorry, not the Open AI SDK. I'm going to

[85:59]
use the Versel AI SDK with Open Router.

[86:02]
Here is a small example of how can I

[86:04]
mean I hope you know what Open Router is

[86:06]
by now. Just go over here, sign up, put

[86:08]
some money in here. So, I've put around

[86:10]
$20 in it. Um and now I can interact

[86:13]
with any model using a single API

[86:14]
interface. That is what open router as

[86:16]
the name suggests provides you. Um you

[86:19]
can use it independently. It tool

[86:20]
calling becomes a little easy if you are

[86:22]
using it with the vers SDK. So that's

[86:24]
what I'm doing. Um I've added two

[86:26]
dependencies to my codebase. That's at

[86:29]
open router/IDK

[86:31]
provider and one add AI which is the

[86:33]
vers AI SDK. If you use both of these uh

[86:37]
boy,

[86:42]
if you use both of

[86:46]
these, it becomes a little easy to

[86:49]
interact with a model. Uh let me share

[86:51]
how to do that exactly.

[86:55]
I need all of this code to be handy

[86:56]
because I need to eventually add a bunch

[86:58]
of tools of our own. Um but the code

[87:00]
that they provide, let's just read

[87:01]
through that really quickly. Um let's

[87:03]
say you want to do an LLM call. Um you

[87:06]
can do that. If you want to get the

[87:08]
weather of a specific city through an

[87:10]
LLM, you number one need to talk to an

[87:12]
LLM. Number two need to provide the LM

[87:14]
with the current weather data because

[87:16]
the LLM unfortunately doesn't have

[87:18]
access to it. Similarly in our case we

[87:20]
need to allow the LLM to be able to open

[87:23]
trades on our account which is only

[87:24]
possible if the LM is able to interact

[87:27]
with our back end and there are various

[87:29]
ways to do it. You can ask the LM to you

[87:30]
know return a specific response in a

[87:32]
JSON format with the positions it wants

[87:34]
to close and open. Um or you can ask the

[87:37]
LM um to uh do a bunch of tool calls.

[87:40]
Tools are basically u backend functions

[87:43]
that the L&M can call based on what it

[87:46]
wants to do. In our case, the thing that

[87:47]
it would want to do would be to open

[87:48]
positions or close positions. So we

[87:52]
instantiate a new instance of open

[87:53]
router using this create open router

[87:56]
function that comes from at open router/

[87:58]
AIDK provider. That's because we're

[88:01]
using the versel AI SDK. If you're

[88:04]
calling them directly then this becomes

[88:06]
a little easier and hard at the same

[88:07]
time. Uh tool calling becomes a little

[88:09]
hard which is why this is the approach

[88:10]
that we're taking. Um let me

[88:14]
replace this with

[88:16]
process.n n dot

[88:20]
uh open router API key. We want to get a

[88:23]
response from a specific model. So that

[88:26]
should probably be it is an input. It

[88:29]
should have been an input as well. Um

[88:30]
and we'll probably restrict ourselves to

[88:32]
you know

[88:35]
quen

[88:37]
or deepseek

[88:39]
or claude. These are the three uh lms

[88:43]
that we're going to use. I need to get

[88:44]
the exact ids of these from open router.

[88:46]
Um let me quickly do that.

[88:55]
So there is anthropic slash

[89:03]
claude 3.5. So that there is uh where do

[89:06]
I get all the list of the models?

[89:22]
We need [snorts] Quinn.

[89:26]
Uh sorry QWEN.

[89:31]
Uh this seems like the best one or the

[89:33]
biggest 110 billion tokens. It's

[89:36]
probably going to be this.

[89:38]
Uh and the third one is going to be

[89:41]
Deepseek R1 which will probably be I'm

[89:45]
assuming Deep Seek/ DeepSync R1

[89:48]
something like that. Uh

[89:51]
how much is this? Uh how many tokens is

[89:53]
this the right one? Uh let's see.

[89:58]
128 billion tokens. That looks fine.

[90:01]
Looks like the biggest one. Um,

[90:04]
copy and paste. All right. These are the

[90:08]
three models that we're going to

[90:09]
support. Um,

[90:12]
the prompt is where we have to do most

[90:14]
of our magic. So, let me just get this

[90:16]
from

[90:18]
another file called prompt.ts.

[90:25]
export const prompt equal to this is

[90:28]
going to be really long now. Um,

[90:32]
you are an expert trader.

[90:38]
You

[90:40]
were given a

[90:44]
$1,000 to trade with.

[90:48]
You are trading on

[90:52]
the crypto market. you are giving the

[90:53]
following information.

[90:58]
I guess it'll just figure it out

[90:59]
whenever we give it the indicators. So

[91:01]
we don't have to provide that

[91:02]
specifically. We have to tell it though

[91:05]
uh you are you have been invoked

[91:10]
invocation

[91:13]
times times. Um the current

[91:18]
open positions are these

[91:21]
uh

[91:25]
your current portfolio

[91:28]
values. This

[91:30]
financial

[91:32]
information

[91:35]
for intraday

[91:38]
5 minute candles.

[91:41]
And here we'll put the

[91:44]
intraday

[91:46]
positions

[91:48]
followed by uh long-term for our candles

[91:50]
long-term positions

[91:52]
and you have in our case we could also

[91:55]
do JSON uh we could ask it to respond

[91:57]
back with JSON or you can have the

[92:01]
create trade or the close position

[92:05]
actually create position

[92:07]
and close position tools to create or

[92:10]
close a position. Um, you can open

[92:15]
positions in one of three markets,

[92:20]
BTC,

[92:23]
ETH, and then sold. Uh,

[92:28]
you can only open one position at a

[92:29]
time. You only one at a time. You can

[92:31]
only get a position if you have enough

[92:32]
money to cover the initial margin. Uh,

[92:35]
that is not needed. You can only get a

[92:37]
position. You don't need that.

[92:40]
Yeah, let's clear off everything else.

[92:41]
This is good enough. Um, we will of

[92:43]
course enrich this prompt a little more

[92:44]
eventually. For now, this is good. Um,

[92:47]
we have a prompt

[92:49]
that we have we import over here. We

[92:51]
just need to replace the prompt with a

[92:53]
few things. Um, invocation times will

[92:57]
come from a DB eventually. Um,

[93:01]
yeah. Um, and also this will also happen

[93:04]
for a model name and an API

[93:09]
key. Uh because

[93:13]
actually we need a global function const

[93:17]
supported accounts equal to um we'll

[93:21]
have three accounts account index

[93:25]
indexana uh no

[93:29]
API key which is going to be

[93:35]
API key underscore uh let's say claude

[93:40]
name which is the claude account. These

[93:43]
are our uh lighter accounts. I'm going

[93:45]
to have an account for claude, an

[93:46]
account for co and an account for uh

[93:49]
deepseek account index is not needed. Uh

[93:52]
but the model name what model to

[93:54]
interact with is needed which is going

[93:56]
to be

[93:58]
this for the first one. Sorry, claude

[94:00]
kill is going to be this.

[94:07]
I think that's good enough. And then we

[94:08]
need a second one which is going to be

[94:10]
deepc. Third one is going to be quen.

[94:12]
And the inputs are going to be you know

[94:14]
let's call this

[94:20]
account array and then let's create

[94:23]
interface account which looks something

[94:26]
like this.

[94:27]
We should probably

[94:30]
not have this here.

[94:32]
You should have this in a separate

[94:35]
file called supported accounts or

[94:37]
something like that. U

[94:40]
accounts is fine.

[94:43]
Let's export this.

[94:46]
Let's export this.

[94:49]
And here we're going to say the function

[94:51]
name is going to be uh invoke lm

[94:56]
or invoke agent is probably a better

[94:57]
word. Um and receive an account as an

[95:00]
input.

[95:05]
Nope.

[95:09]
Here we're going to give it the account

[95:11]
domodel name.

[95:14]
Before we talk to the LM, we need to get

[95:16]
all of our open positions, things like

[95:17]
these, and all the indicators. So, con

[95:21]
indigators equal to get indicators.

[95:26]
uh

[95:28]
let's call them intraday indicators

[95:30]
long-term indicators that's fine um

[95:34]
const open positions is going to be this

[95:36]
and then in our prompt

[95:40]
I'm going to replace invocation times

[95:42]
we'll have to see um this will come from

[95:45]
a DV I'm going to keep this as

[95:48]
zero for now open positions

[95:53]
okay let's just put this in the next

[95:55]
line so Let's make a little bit more

[95:56]
readable.

[96:02]
Open positions is

[96:14]
we'll see how to format this soon

[96:16]
enough. Boy, what what is up with this?

[96:26]
Then lastly, replace over here.

[96:30]
And what's the problem here?

[96:36]
Possibly undefined. Um,

[96:38]
question mark here

[96:41]
should do. And portfolio value.

[96:45]
Oh boy, we need another endpoint, don't

[96:47]
we? To get the current portfolio value.

[96:51]
Yeah, shoot. I forgot another endpoint,

[96:54]
which would be let me just make it

[96:57]
$1,000 for now. But eventually, we need

[97:00]
to get the current balance of the

[97:03]
account or the current, you know, money

[97:05]
that the account has, including the open

[97:08]
position values plus the collateral that

[97:10]
the user still has. And intraday

[97:12]
indicators. What's the problem here?

[97:15]
Long-term indicators.

[97:19]
This looks fine to me.

[97:21]
Yeah, this looks fine. Um,

[97:25]
open a position in the given market.

[97:29]
Open. That's the first tool that the LM

[97:31]
can call.

[97:33]
And then there's another tool for

[97:35]
closing the position which we'll come to

[97:36]
eventually.

[97:38]
Return

[97:44]
position opened

[97:47]
successfully at an average price of

[97:53]
or this quantity the symbol that looks

[97:54]
fine to me.

[97:56]
Uh we'll get the price quantity symbol

[97:58]
soon enough. Um the inputs are going to

[98:02]
be price quantity symbol looks fine to

[98:05]
me but it'll not be probably not the

[98:08]
price. It'll just be a long or a short.

[98:09]
We don't care. We'll just place it at

[98:11]
the market. U quantity is needed. Um

[98:16]
symbol is needed and then side is going

[98:19]
to be long or short.

[98:22]
Leverage is going to be a number. Uh

[98:25]
which is you know do you need 5x

[98:27]
leverage 10x leverage so on and so

[98:29]
forth. Um that is good enough. Uh

[98:34]
oh sorry this will be a Z enum

[98:37]
of long or short

[98:42]
and it's going to be an array

[98:45]
and leverage is a number price is not

[98:48]
needed

[98:50]
and here I'm going to replace these with

[98:52]
these and here is where I have to the

[98:56]
execute function we'll figure out soon

[98:59]
enough let's do the same for closing a

[99:01]
position Close

[99:05]
position

[99:07]
should give us parameters.

[99:11]
How do you cancel an order? I think I've

[99:13]
written this as well.

[99:20]
Examples

[99:22]
cancel order.

[99:30]
Oh boy, it's going to be really hard.

[99:35]
Client cancel order. Uh, signer client.

[99:39]
All right, I have to

[99:42]
do a

[99:44]
cancel

[99:47]
order

[99:50]
takes a market index as an input. Uh,

[99:54]
that's not great. The problem, not the

[99:56]
market index, sorry. Um the client order

[99:59]
index.

[100:02]
Uh yeah, that's going to be really

[100:04]
tricky. Um canceling an order is going

[100:06]
to get a little hard, but it I'll just

[100:10]
ask it for the symbol. But we'll tell

[100:12]
the LLM in the prompt

[100:16]
position. You can only close the

[100:20]
position fully. you can't edit an

[100:24]
existing position slashc close the

[100:27]
position partially. Um I'll have to add

[100:29]
this constraint. Um that's purely

[100:32]
because when we are placing an order um

[100:34]
it's very hard to find that specific

[100:36]
position and order and cancel it. Um but

[100:39]
closing the whole position is a little

[100:40]
easy. Um

[100:43]
so yeah, I'm going to stick to this for

[100:45]
now. Um yeah, actually not even this. Um

[100:48]
you can open a position at a time. You

[100:50]
can

[100:52]
close all open positions

[100:55]
uh with the close position tool. You can

[100:58]
not close/edit

[101:01]
individual

[101:02]
positions. All existing

[101:07]
positions must be cancelled um

[101:12]
at once.

[101:15]
Even if you want to create only to

[101:21]
close one position, you must call option

[101:24]
one and then reopen

[101:28]
positions that you want to keep. That

[101:30]
looks fine to me. Um

[101:33]
yeah, now it's good. So close position

[101:34]
can simply be or close position

[101:37]
doesn't need any inputs. Um an empty

[101:40]
object is fine. And this should just

[101:42]
call the cancel all function that we

[101:45]
already have. Um, this is fine.

[101:49]
This is

[101:51]
you can call it create position. Create

[101:53]
position. It should probably be called

[101:55]
close all positions.

[101:58]
Close all the

[102:01]
currently

[102:03]
open positions. Um, will not have any

[102:06]
inputs. All position close successfully.

[102:09]
All right. I see a bunch of

[102:12]
red squiggies.

[102:13]
Oh, this shouldn't be called parameters.

[102:15]
It should be called

[102:17]
input schema. Um, we should change that

[102:20]
in the documentation

[102:22]
wrong over here. Um, is there a way to

[102:25]
edit this page?

[102:27]
Uh, this one right here. This is

[102:29]
incorrect. It should have uh input

[102:33]
schema as the argument here and not

[102:34]
parameters.

[102:36]
If there was a way to edit it, I would

[102:38]
edit because I've been stuck on this a

[102:40]
few times. I don't see a way to edit it.

[102:41]
All right. Um, cool. Create positions.

[102:44]
Position open successfully. And then

[102:46]
close position. Okay. We do need the

[102:48]
current price.

[102:53]
Okay. Let's just write the logic to open

[102:55]
a position also over here. Um,

[102:59]
thinking thinking.

[103:02]
Yeah, what we can do is um

[103:08]
I think this is in the examples as well.

[103:12]
There should be a create position or

[103:14]
create order function. Um let's see.

[103:18]
Yeah, we need to put this.

[103:22]
We haven't yet written this. Um we've

[103:24]
written the logic to get the indicators

[103:25]
and get the

[103:29]
the other thing which is the open

[103:30]
orders. create position.ts

[103:36]
we haven't yet created. So let's see uh

[103:39]
there should be a function called

[103:42]
create position

[103:44]
that will take the symbol side quantity

[103:47]
leverage um

[103:49]
s symbol side quantity is good enough

[103:51]
leverage is something

[103:54]
do we need the lm to tell us leverage

[103:56]
probably not if we tell it already ka

[103:59]
leverage it'll just increase the

[104:00]
quantity by that much so I think we'll

[104:03]
be fine if we don't ask the lm to give

[104:05]
us the leverage symbol side quantity is

[104:08]
good enough. The other thing we need is

[104:11]
um the API key. Basically the account is

[104:13]
good enough. I think should probably

[104:15]
take the account as an input here. We

[104:19]
should create a new ser client with the

[104:22]
base URL which is fine. Um private key

[104:26]
comes as an input. So

[104:29]
accounti

[104:31]
key what's the problem here?

[104:36]
I've imported account from the wrong

[104:38]
place. Let me just import it from the

[104:40]
right place.

[104:43]
There we go. Uh

[104:46]
symbol string is well okay this is where

[104:49]
we need to add another thing to

[104:50]
accounts.ts

[104:52]
uh which is oh

[104:55]
not just accounts.ts. Let's create a new

[104:57]
file called

[104:58]
markets.ts Yes s export con markets

[105:02]
equal to uh BTC

[105:05]
which has I think ID one BTC key

[105:11]
and don't need any of this.

[105:15]
Let's make it the market ID. Oh, we'll

[105:19]
need a bunch of other things here.

[105:20]
Decimals and things like these.

[105:22]
Decimals.

[105:23]
Let's say I don't know what the decimal

[105:26]
count would be here. Decimal is people

[105:28]
who have done trading know this. People

[105:29]
who don't haven't done trading. Um when

[105:31]
you forward a an order to any DEX um or

[105:35]
a centralized exchange, you don't tell

[105:37]
it you want to buy one BTC. You some

[105:40]
sometimes have to tell you have to buy

[105:42]
this much BTC. This actually means one

[105:44]
um or if you want to buy uh you know uh

[105:49]
it at a price of 100K u and then you

[105:51]
have to give it a price that looks

[105:54]
something like this. Basically, you have

[105:55]
to give it an integer and not a

[105:58]
floatingoint number. And what should you

[106:00]
multiply by is what decimals represents.

[106:03]
If decimals is 1 10,000, um then if you

[106:06]
want to place an order for 0

[106:08]
01 BTC, you'll actually place an order

[106:10]
for 0.01 into 10,000, which is 100. Um

[106:13]
if you want to place an order at 100K,

[106:15]
then you'll actually place an order at

[106:17]
100K plus 4 zeros. U so the decimal

[106:21]
field which we'll have to see what it is

[106:23]
for BTC ETH and soul u represents okay

[106:25]
how much do you need to amplify the

[106:27]
quantity and the price by um this looks

[106:30]
good to me I think ETH market index was

[106:32]
zero and then sold was this thing um in

[106:36]
the prompt I should probably tell it

[106:40]
BTC sold that's fine

[106:43]
symbol

[106:45]
and identifier

[106:47]
actually I don't have to tell it here in

[106:49]
the tool call uh which was done over

[106:51]
here BTC itself that looks fine to me um

[106:55]
but actually yeah it was telling the

[106:57]
right thing which was markets dot

[107:02]
keys that looks fine

[107:08]
uh object dot keys of markets

[107:12]
looks better um long short and the

[107:15]
number

[107:18]
The quantity

[107:20]
to open the position at the quantity

[107:27]
of the position to open

[107:33]
doesn't need to multiply by the decimal

[107:35]
u but let's not specify that here coming

[107:38]
back to create position.ts Yes, account

[107:41]
symbol side and quantity

[107:44]
API key index is

[107:48]
sort of kept it as two. We should just

[107:50]
define it in a single place though. So

[107:52]
I'm going to create another file called

[107:54]
config.ts FTS

[107:57]
where I'm going to export this export

[108:02]
and I think there was another place we

[108:03]
were using this which was close order or

[108:06]
cancel order where we should probably

[108:08]
just

[108:10]
import this

[108:14]
and private key is it needed at all.

[108:17]
What is this main function?

[108:20]
Wait, this is the wrong file. My bad.

[108:22]
This is an example from the SDK.

[108:26]
What was our file that I just wrote for

[108:30]
canceling an order? Create position

[108:33]
and then accounts.tss. No, that's not

[108:35]
the one. Stock data. That's not the one.

[108:38]
Open markets indicators.

[108:42]
Create position.

[108:46]
Which one was it? Uh,

[108:50]
open positions. There you go. Um,

[108:53]
this needs to be imported.

[108:58]
Perfect.

[109:01]
Hold on. Is this the account index or

[109:03]
the API key index?

[109:07]
Oh, sorry. It's the account index.

[109:16]
Could have sworn there was an API index.

[109:17]
Never mind. Uh, even the account index

[109:19]
needs to be in config.ts. TS

[109:28]
30967.

[109:30]
That looks fine. So, let's just import

[109:32]
that from

[109:34]
the config. All right, [clears throat]

[109:36]
we're good to go. Coming back to uh

[109:39]
create position.ts.

[109:46]
Uh sold market ID not needed.

[109:50]
API key index is going to be

[109:56]
API key index from config. Account index

[109:58]
is going to be account index from

[109:59]
config.

[110:00]
This is a very interesting and important

[110:02]
thing that we need to worry about. Uh

[110:06]
we actually probably don't need to worry

[110:07]
about this uh because the function

[110:09]
barber call if I defined this outside

[110:11]
basically is where this could have been

[110:13]
a problem. If I'm defining it inside an

[110:15]
optimistic non management type is fine.

[110:18]
Um

[110:21]
market ID looks fine to me

[110:25]
and this will exist.

[110:29]
Then we select the client order index.

[110:33]
This is the interesting part um which we

[110:36]
will need when we eventually cancel

[110:39]
orders. When we eventually cancel

[110:41]
orders, the client order index is

[110:44]
needed. When you open a position or open

[110:46]
an order, you give it a client order

[110:48]
index, a client order ID. When you want

[110:50]
to close that position, you have to

[110:52]
repass that client order index.

[110:54]
[clears throat] What we can do is we can

[110:56]
have it to be zero for all BTC orders

[111:00]
and have it to be one for all ETH

[111:01]
orders.

[111:04]
Even though that's a slightly more ugly

[111:05]
way to do it, but I'm just going to do

[111:07]
it for now. Um,

[111:10]
so if you go to markets,

[111:14]
uh,

[111:17]
client order index is zero. And for all

[111:19]
the ETH orders, I'm going to have it as

[111:21]
one. For all the sold orders, I'm going

[111:23]
to have it as two. And then I can simply

[111:27]
do the same thing over here. Base

[111:30]
amount.

[111:33]
Okay.

[111:34]
Now comes the most interesting bit.

[111:37]
Firstly you should just do this const

[111:39]
market equal to this mark index is fine

[111:43]
client order index is fine base amount

[111:45]
of llm will give us one soul we probably

[111:49]
need to multiply it by a number and I'll

[111:50]
have to check what this number is for

[111:52]
all of these three markets the same is

[111:54]
true for the price the lm oh price is

[111:57]
just a [ __ ] we have to get a market

[111:59]
order right not a limit order but create

[112:04]
order unfortunately ly

[112:09]
oh

[112:11]
SDK SDK

[112:14]
create order is

[112:18]
a limit order and not a market order

[112:24]
create market order is there one

[112:32]
thing it's not there uh all righty um

[112:35]
Here we can basically put obnoxious

[112:36]
prices if you want. Um

[112:39]
yeah uh

[112:43]
there's another way to do it. We can

[112:44]
create a a market order but let's not go

[112:46]
there. Here we will have to take an

[112:49]
obnoxious price. So I'm going to add

[112:51]
another field here called

[112:54]
obnoxious

[112:58]
price which is going to be let's say 300

[113:00]
for

[113:02]
Solana.

[113:05]
Basically price at which an order will

[113:07]
definitely be available is this

[113:09]
obnoxious price and then obnoxious price

[113:14]
for

[113:17]
oh wrong

[113:20]
place.

[113:22]
I'm thinking if I want to do this the

[113:24]
cleaner way. I want to. I just don't

[113:26]
have the time which is why I'm doing it

[113:28]
the wrong way. Um obnoxious price 300.

[113:31]
No, for BTC it's probably 200k. 200,000.

[113:35]
Mhm. For ETH, obnoxious price is

[113:38]
probably

[113:39]
$7,500. Very low probability it reaches

[113:42]
this price in [snorts] the next 10 days.

[113:44]
And for soul, the obnoxious price is

[113:46]
going to be uh let's say again $350. Uh

[113:50]
very low probability it reaches this

[113:51]
price in the next 7 days is when we're

[113:53]
going to have the bot active. Um the

[113:55]
reason I'm doing this is because I there

[113:57]
is no way to create a a market order. At

[113:58]
least I haven't exposed that in the SDK.

[114:00]
I can only create a limit order. When

[114:02]
you create a limit order, you have to

[114:03]
give it a price.

[114:05]
If it's lesser, great. But I will I'm

[114:06]
willing to buy it at this price. So

[114:08]
that's why this is an obnoxious price

[114:10]
that I'm putting over here. So that I

[114:12]
place a limit order, but it's at an

[114:13]
obnoxious price. So it will get matched

[114:14]
at the best price um that is currently

[114:16]
available. Um is ask is a beautiful

[114:19]
thing which will come from the side.

[114:22]
Side will either be long or short. Um

[114:26]
so this is going to be if it is long

[114:33]
then it is a bid that is

[114:37]
so that looks wrong to me. Uh I think it

[114:39]
will be like this.

[114:41]
If it is a long then it'll be a bid. I'm

[114:43]
bidding to long right.

[114:46]
Let's see. Let's confirm that. I'm

[114:47]
pretty sure that is true.

[114:50]
Where is

[114:53]
uh lighter?

[115:02]
Let's see for the Solana market.

[115:06]
Uh if I select a specific price and I

[115:09]
click on long. Sorry, what was I think I

[115:13]
was going on? Long or short. So if I am

[115:15]
buying or longing,

[115:17]
I can select a price, right? So this is

[115:19]
a bid is ask will be false.

[115:23]
My bad. It was correct.

[115:26]
If it is a bid then it is for a long it

[115:30]
should be a bid. So is ask should be

[115:32]
false. That is true.

[115:35]
Order type. Order type. Oh there you go.

[115:38]
Can I just make this

[115:41]
this and are we good to go then? [ __ ]

[115:43]
There was an order type. I didn't need

[115:44]
any of the obnoxious price things. Cool.

[115:46]
Time and force is fine. Reduce only is

[115:48]
fine. um trigger price not needed order

[115:52]
expired 28 days from now is fine

[115:55]
and just export the function that should

[115:58]
create a new position

[116:01]
and account dot ts or sorry index.ts

[116:05]
TSMA. All I have to do is call this

[116:08]
leverage is not needed. And here I will

[116:10]
do a create position

[116:15]
account symbol side and quantity.

[116:20]
This should probably return me the price

[116:21]
and the average price at which the order

[116:23]
was placed and everything. I'm not

[116:25]
worrying about that yet. Um, close all

[116:27]
position also becomes very easy now

[116:29]
because I can simply call close

[116:31]
position.

[116:33]
Do I have a close position or cancel

[116:35]
order? I don't have a function of my own

[116:38]
to cancel order. But I have an example

[116:40]
here.

[116:43]
Cancel order. This thing

[116:47]
that looks fine to me. Let me create

[116:48]
another function here. Another file here

[116:51]
to cancel order.

[116:56]
Sport function

[116:59]
cancel

[117:00]
order.

[117:04]
Come on, give me all the imports

[117:05]
quickly.

[117:14]
Base UI also needs to be in the config.

[117:28]
Private key is fine. Account index is

[117:30]
fine. Uh API index is fine, knowledge

[117:33]
management is fine.

[117:36]
Cancel for

[117:38]
the specific market ID

[117:42]
where

[117:44]
oh

[117:51]
con markets

[117:57]
object dot

[118:01]
values of markets for every market that

[118:03]
we have

[118:06]
cancel

[118:09]
all orders

[118:13]
for every market that we have.

[118:17]
I want to cancel order for that market

[118:19]
ID using that client order index. Is

[118:22]
that the right thing that's done over

[118:23]
here? Marketing index. That looks

[118:25]
perfect to me. The only problem is this

[118:29]
looks like an object here.

[118:33]
This doesn't look like an object.

[118:43]
Oh boy. Is the example over here

[118:45]
incorrect? Looks like it's incorrect. Uh

[118:50]
yeah, the SD example looks incorrect.

[118:51]
But either way, um,

[118:55]
this is fine then. We'll see if it works

[118:58]
or not. Maybe it doesn't work. U, that's

[119:00]
it. That should do it. If I come back

[119:04]
here,

[119:06]
not here in index.ts.

[119:10]
Cancel all orders for

[119:13]
the specific account and close all

[119:15]
positions is what needs to be returned.

[119:16]
So, I think I've done everything. Um, I

[119:18]
actually need to see the prompt. Oh, the

[119:20]
prompt is missing a few things. The

[119:21]
number of invocations is incorrect, but

[119:24]
everything [snorts] else is fine. Tools,

[119:26]
it has a create positions tool. It has a

[119:28]
close all positions tool. This tool

[119:30]
would actually create the position

[119:34]
and return this. This tool would

[119:35]
actually close the position and return

[119:37]
this. All right. Uh, let's change the

[119:39]
prompt a bit.

[119:42]
Oh, one big thing I'm missing right now

[119:44]
is

[119:46]
when you create an order,

[119:50]
uh, is this the right one? Yeah, also

[119:54]
base URL should now come from config.ts.

[119:59]
Um,

[120:02]
this we need to confirm this. How do you

[120:05]
pass the quantity over

[120:10]
and how do you pass the price over? So

[120:12]
for that I'm basically for every market

[120:15]
going to place some orders and see what

[120:17]
price and quantity they pass over here.

[120:20]
So for this Solana market um if I place

[120:22]
a an order for let's say 0.1 soul and

[120:27]
look at the network tab.

[120:30]
Click on place limit order at 193.742.

[120:36]
The transaction that I send has

[120:40]
let's see the payload as base amount

[120:43]
100.1.

[120:46]
So the decimal is probably a,000 1 93

[120:50]
742 93 742. Yeah. So the decimal is

[120:53]
a,000 for

[121:00]
Solana

[121:06]
for the market index three. Right? Let's

[121:09]
see what is the Sana market index. It is

[121:13]
gone

[121:15]
two. My bad. Sana market ID is two.

[121:20]
Let's go to Ethereum.

[121:23]
Let's place a small Ethereum order as

[121:25]
well. Let's close the old one. It's down

[121:27]
14%.

[121:31]
Oh, it's a old order. My bad. This is an

[121:33]
open order that I'm going to close as

[121:34]
well. Um, for Ethereum, I'm going to now

[121:38]
again place an order at some 3,900

[121:42]
price. Let me just select 3,000 here so

[121:44]
the position doesn't open. at 3,000 I

[121:47]
want to log let's say 0.1

[121:49]
[clears throat]

[121:50]
place limit order

[121:53]
and if you look at the transaction that

[121:55]
is sent the price is 3,000 into 100 and

[122:01]
the quantity 0.1 is 1,000 oh boy. So we

[122:06]
basically need two sets of decimals. Um,

[122:09]
we need the price decimal

[122:15]
and then we need the

[122:17]
quantity

[122:21]
decimals. Um, the price decimals here is

[122:25]
1 2 3 4 5 600.

[122:29]
The quantity decimal here is

[122:33]
1,000.1. So that's four zeros.

[122:38]
And here both were a,000.

[122:43]
Now let's see what Bitcoin provides us

[122:46]
with. Let me close this open order

[122:50]
for BTC.

[122:52]
Uh if I

[122:59]
Okay, it's fine.

[123:02]
Try to place a limit order at let's say

[123:04]
100K.

[123:07]
100k with the quantity of 0.1

[123:14]
uh place a limit order.

[123:18]
If I look at the request that goes out

[123:22]
so that's 01 and then 100k at 100k. So

[123:28]
001 is 1 2 3 4 5 quantity to it is five

[123:34]
zeros

[123:37]
and price decimal is okay what's the

[123:40]
price here 100 1 2 3 100 1 2 3 4 10

[123:48]
uh price decimal is 10 that should do it

[123:50]
and the market ID was one right for BTC

[123:53]
the market ID is one perfect for

[123:55]
Ethereum the market ID is zero all right

[123:57]
I think we're good to go. I think we're

[123:59]
sending the right inputs now. Um, let me

[124:01]
close this.

[124:03]
Let me change the prompt to tell it,

[124:05]
hey, you've only been given $50, not

[124:08]
$1,000 to trade with right now. Only

[124:11]
$50. Thank you. Um, with these $50,

[124:15]
the LM is supposed to trade. Um,

[124:19]
and we do have $50 here, I think, in my

[124:22]
portfolio. I do have $50. Uh, if I go to

[124:26]
tools and API keys, I don't have an API

[124:30]
key yet for [clears throat]

[124:33]
this Quen account. I think I currently

[124:35]
I'm in my Quen sub account. Let's see.

[124:37]
API keys. Sorry. Uh, sub accounts.

[124:42]
I'm currently in the DeepSeek. My bad.

[124:44]
Deepseek account, which has some $49.

[124:47]
Um,

[124:48]
I need to go back here. I need to go

[124:51]
back

[124:53]
here. Generate an API key. Add the

[124:56]
account index of two. Oh god, I need to

[124:58]
sign this. Never mind. Let me just do

[125:01]
the first sub account. So, let me go

[125:03]
back to my sub accounts. Select the

[125:06]
first one. I already have the API key

[125:08]
for this here. Um, let me

[125:13]
rename this to

[125:17]
uh

[125:26]
crowd.ts

[125:28]
API key cla.

[125:31]
Yeah, now this is very confusing. This

[125:33]
is not the claw API key. This is a

[125:34]
lighter claw account API key. So, I

[125:36]
should rename this to something else.

[125:38]
It's fine for now. And now let me do a

[125:42]
supported accounts of zero pay. Also

[125:45]
need to add the open router API key. So

[125:47]
let me go to env.

[125:49]
Let me create a new open router API key.

[125:51]
Uh which will come from

[125:54]
here. You guys can see it right now. I

[125:56]
will change it. Delete

[126:00]
this ASAP.

[126:04]
Copy it. Paste it. And I think we're

[126:06]
good to go. I mean, I don't know if

[126:07]
we're good to go. Let me just log this

[126:09]
for now. Let me not create a position.

[126:12]
Let me not cancel the orders. Let me

[126:14]
just log. Console.log. Position

[126:16]
successfully created. And then

[126:19]
console.log. All position closed

[126:20]
successfully.

[126:22]
And

[126:23]
my portfolio value is 50 to begin with.

[126:27]
And we'll slowly introduce the database

[126:30]
um to track the invocation times and the

[126:32]
portfolio value. Let's do a bun index.ts

[126:35]
Yes. And let's see what happens.

[126:38]
Did I call the function? I did not. So

[126:40]
let's call also invoke agent once um

[126:45]
with the very first account that is the

[126:48]
clawed account. And let's see what the

[126:50]
clawed account

[126:52]
responds back with um this is fine.

[126:59]
Wait for it.

[127:04]
The indicators were calculated. That's

[127:06]
where the these logs came from.

[127:09]
And I'm just waiting for the tool calls.

[127:13]
No tool calls. Interesting.

[127:17]
Return console.log.xsl.

[127:27]
Wait for it.

[127:33]
position opened successfully for 0.004

[127:37]
E. All positions close successfully

[127:40]
right after [clears throat]

[127:42]
that is not great. Um

[127:45]
like it opened

[127:49]
a new position to long ETH. Great. But

[127:52]
then it just closed all the open

[127:55]
positions immediately after which is not

[127:58]
great. I probably need to log a bunch of

[128:00]
other things now.

[128:06]
Uh let me go a consideration. We have a

[128:08]
short position in ETH currently open. No

[128:11]
you don't. Uh looking at the price

[128:13]
action intraday slightly upward trend.

[128:15]
Long-term slight significant downward

[128:17]
trend. Um given that we already have we

[128:22]
have a short position. We might have

[128:24]
one. Uh let's see. Uh

[128:28]
but we do have a short position. It

[128:30]
wasn't incorrect. It's down 19%. We do

[128:33]
have one. Okay. I mean close that.

[128:38]
Uh I don't recommend making any changes.

[128:40]
All right. Now let's try that one more

[128:42]
time with no open positions. I mean

[128:43]
there was an open position. Um, but

[128:45]
let's say I do the same thing now with

[128:46]
no open positions.

[128:50]
You'll probably long ETH at this point.

[128:55]
I'll recommend opening a small position

[128:56]
in ETH. And if you look at

[129:00]
there are no tool calls that happened

[129:01]
though. Oh, position opened successfully

[129:03]
for 0.0125 ETH. Uh, do we have enough

[129:07]
margin for that? So 0.0125 E.

[129:11]
Uh, we might not.

[129:15]
0125 into 4,000. Yeah, it's like $50

[129:20]
exactly, which is not great. It should,

[129:23]
you know, probably send a more

[129:25]
conservative position, but it's actually

[129:27]
also not [snorts] $4,000. We could have

[129:29]
actually opened that position. Let's

[129:30]
just uncomment and let's just see if it

[129:33]
actually creates the position or not.

[129:38]
All right, moment of truth.

[129:52]
Okay.

[129:54]
All positions closed successfully

[129:59]
followed by the prompt

[130:02]
and then oh it only sent a request to

[130:05]
close the position which probably means

[130:06]
there's no position. Doesn't no

[130:08]
transaction data returned from ser.

[130:20]
This is not great. Uh

[130:23]
yeah, we'll have to figure out I'll

[130:24]
basically have to update the lighter SDK

[130:26]
now to support market orders. Uh there

[130:28]
is no other easy way to fix this

[130:29]
unfortunately. Um the good thing is it

[130:31]
shouldn't be too hard for me to support

[130:33]
market orders. Let me spend some time

[130:35]
there. update the SDK, make an open

[130:37]
source contribution u to my own repo and

[130:40]
then go from there. Uh

[130:43]
let me just close this order

[130:46]
and

[130:47]
yeah, let me be back in I don't know

[130:50]
probably 20 minutes. In 20 minutes, I'm

[130:52]
going to update the lighter SDK. Once I

[130:54]
do that, we should be able to get

[130:56]
support an order here that is of type

[131:00]
market. Uh right now, we only support

[131:02]
limit orders. All right, going to do

[131:04]
that really quickly and be back.

[131:08]
All right, that took a long time. Uh,

[131:11]
there were a few things that I had to

[131:13]
change. I'll take you through everything

[131:15]
that I've changed in the last 1 hour or

[131:17]
so. When creating a position, um, the

[131:21]
thing that was wrong before this was, if

[131:23]
you remember, we had this thing called

[131:25]
the obnoxious price um, for every

[131:29]
market. uh we don't need that anymore.

[131:32]
But what was happening was um if even if

[131:36]
you place a market order um if you place

[131:37]
it at an obnoxious price, right? If you

[131:39]
say I want to buy Solana um and I'm

[131:44]
willing to buy it at $7,500. In that

[131:46]
case, if you get Solana for $300, the

[131:50]
order goes through. But if you say, "I'm

[131:51]
willing to sell it at $7,500. Um but the

[131:53]
current price is 200 or 300. Um the

[131:56]
engine wouldn't let it through. Even

[131:57]
though it's a market order, market

[131:58]
orders are supposed to just go in at any

[132:00]
price. There's still some slippage um

[132:02]
that the engine would have, you can

[132:03]
probably change that slippage over here,

[132:05]
but by default, the slippage would

[132:06]
probably be 1%. So, I can't just give a

[132:09]
an obn obnoxious price over here, which

[132:11]
is what we were doing before because if

[132:13]
I give it an obnoxious price, even

[132:15]
though it's a market order that we're

[132:16]
placing, it's not a limit order. Um the

[132:18]
market order wouldn't go through if the

[132:19]
price is too far off from the current

[132:21]
price. So what I've changed over here is

[132:22]
um can I get the current price using the

[132:26]
candlestick API that we already had? Um

[132:28]
I basically

[132:30]
Oh, that's the wrong one. Sorry. Create

[132:33]
order is over.

[132:37]
Oh,

[132:39]
create order

[132:42]
or open position. Yeah, if you look at

[132:44]
the this one, if you look at the create

[132:49]
position.ts ts file. Now, um

[132:53]
before sending an order to create a

[132:56]
position uh or create an order, I first

[132:58]
get the latest candlestick using the

[133:00]
candlestick K that we already had. 1

[133:02]
minute chart, just one candle. Um and we

[133:05]
get the current price, the close over

[133:07]
there would just tell us what is the

[133:08]
latest price at which Salana was or

[133:10]
Ethereum was bought or sold. Um, and

[133:12]
when we're placing the actual order, um,

[133:15]
we based on if it's a long or a short

[133:17]
position, we take a price slightly more

[133:19]
or slightly less. If it is a long

[133:20]
position, I tell the trading engine, um,

[133:23]
I'm placing a market order and I'm

[133:25]
willing to pay the current Salana price

[133:27]
plus a little bit more. And if I'm

[133:28]
shorting, if I'm selling, I tell the

[133:30]
order engine, okay, I'm willing to get

[133:32]
back the current Sana price or a little

[133:34]
bit less.

[133:36]
Earlier, we had an obnoxious price over

[133:38]
here. So when you would say I want to

[133:40]
buy or if I when you would say I want to

[133:41]
sell at $7,500 when Ethereum is at

[133:44]
$4,000 it wouldn't let you place a

[133:46]
market order because the the delta is

[133:48]
too much. Um and you know there's only a

[133:50]
little bit of delta that would allow you

[133:52]
to u to have irrespective of it being a

[133:55]
market order. Um so that's the one thing

[133:57]
that I changed. Um the second thing that

[133:59]
I changed was um in the cancel function

[134:04]
in the cancel order function um not this

[134:07]
one again

[134:12]
not this one this one. Um earlier I had

[134:15]
I was just calling a cancel function

[134:17]
that I exposed from the lighter SDK.

[134:19]
There's nothing wrong with that also.

[134:21]
You can do that as well. The problem

[134:22]
there was uh I was placing we we were

[134:25]
placing cancel orders for markets in

[134:26]
which we did not have any open orders as

[134:27]
well. We just had a for loop over here.

[134:29]
What I've changed that to is um I first

[134:31]
get all the open positions that we

[134:33]
currently have. We already had written

[134:34]
this function that gives you all your

[134:36]
open positions. Basically everything

[134:37]
present over

[134:39]
here. All of these open positions um and

[134:41]
we just place an opposite order um for

[134:43]
those open positions. So if there was a

[134:45]
10x long with a quantity of one on

[134:47]
Ethereum, we place a 10x short um for

[134:50]
the quantity one for Ethereum so that

[134:52]
the order just cancels. Um what do we do

[134:55]
rather than canceling an existing order

[134:56]
because there there things get a little

[134:58]
tricky with the ids that you're sending.

[134:59]
Um you if you remember we had this

[135:02]
concept of uh

[135:05]
client order index. We don't need that

[135:07]
anymore because earlier I had this so

[135:10]
that I could keep track of what I need

[135:12]
to send when I'm canceling an order. Now

[135:14]
I don't really cancel an order. I just

[135:16]
create an opposite order. If there is a

[135:17]
long that is present in my open orders,

[135:19]
I just create a short that ends up

[135:20]
canceling the position over here and the

[135:22]
order sort of goes away. That is the

[135:24]
second thing that I've changed. I've

[135:26]
tested both the functions. They seem to

[135:27]
be working. Um let me show you what the

[135:31]
script does right now. So if you look at

[135:32]
index.ts, um it's still the same

[135:35]
function. We still have to change a few

[135:36]
things. I've hardcoded the invocation

[135:38]
times and the portfolio value right now.

[135:40]
We do need to change that. Um but other

[135:43]
than that, if I do a bun index.ts TS.

[135:45]
Now you will notice

[135:48]
it is number one able to place trades.

[135:51]
Um let's see what the AI responds with

[135:52]
and whether or not that trade gets

[135:54]
created. Um

[135:57]
the responded to something that specific

[135:59]
trade was created over here. 0.1250 ETH

[136:02]
short. And if you look at the response

[136:04]
it'll probably tell you something

[136:06]
similar. Um

[136:11]
take a short position on ETH. Um the

[136:13]
tool call is not logged over here but

[136:16]
I'm assuming it just asks to do a 10x

[136:18]
short on Ethereum with this much

[136:20]
quantity 0.1250 ETH. Um and then we can

[136:23]
wait for a long time a short time

[136:25]
doesn't matter. This time when we run

[136:26]
the script the LM will also be aware of

[136:28]
the current open orders and it might

[136:30]
cancel this order. I would want for it

[136:32]
to cancel this currently open order so

[136:34]
that um I can you know uh see the cancel

[136:38]
order. I can show you the cancel order

[136:40]
function is also working. Let's see if

[136:41]
that happens.

[136:43]
Uh so right now respond to I will not

[136:45]
make any changes in the trades. Um just

[136:48]
to show you that the cancel order

[136:50]
function actually works. I can actually

[136:52]
call it over here. If I call cancel all

[136:55]
orders um and run it again

[137:00]
as you saw the order sort of went away

[137:02]
from here. So whenever the tool call for

[137:04]
canceling an order happens we can see it

[137:06]
does work as well.

[137:09]
Now this pretty much works. Now I can

[137:12]
run a chron job that runs this every 5

[137:14]
minutes, 10 minutes. Um it will have

[137:16]
access to the open positions. It'll have

[137:18]
access to it doesn't have access to the

[137:19]
portfolio right now. One thing I would

[137:20]
like to add over here is making the LM

[137:23]
aware. Okay, this is your current

[137:24]
profits and current losses. So your net

[137:26]
portfolio position is something I would

[137:28]
like to return. I'm unsure what endpoint

[137:30]
we'll get that from. That's one thing to

[137:31]
add. The second thing is um a bunch of

[137:33]
database logic. I need to log everything

[137:35]
to a database. Uh what positions were

[137:36]
taken at what times. Um and then we need

[137:38]
to maintain a time series graph of u

[137:41]
what is the portfolio at you know let's

[137:43]
say every 5 minute interval so we can

[137:44]
create a graph finally to how track how

[137:46]
the position is performing. Um let's do

[137:48]
that next. Let's get into basically some

[137:50]
fairly standard web two u stuff. We'll

[137:53]
add uh a database. We'll add Prisma.

[137:56]
We'll write the schema for it. We'll

[137:57]
write a process that is constantly

[137:59]
logging the uh the candlesticks. And

[138:02]
then eventually we'll write a front end

[138:03]
that you know lets you display the

[138:04]
candlesticks and the portfolio

[138:05]
positions. One more thing I've done is

[138:07]
I've on rammed around $3,000 to this

[138:09]
account. So by the time this

[138:13]
um deposit finishes, we will have 3,000

[138:16]
accounts. Um we already have three sub

[138:18]
accounts over here. We have two of them.

[138:20]
Let me add another one for Claude.

[138:25]
Um I'll just put $3,000 in all three of

[138:27]
these. And once we do that, we'll just

[138:30]
let the script run overnight and we'll

[138:32]
see u where we reach tomorrow. All

[138:35]
right. I'm going to move to the database

[138:37]
logic next.

[138:40]
All right, time to code things

[138:43]
we're already good at. That's web two.

[138:46]
So, let's quickly add Prisma as a

[138:50]
dependency.

[138:52]
Let's start docker locally.

[139:00]
Let's do a bun express in it.

[139:04]
Let's write the schema for our

[139:06]
application today.

[139:17]
There we go. Oh, Prisma keeps changing

[139:20]
very quickly. Uh model

[139:24]
there would be a

[139:27]
[snorts]

[139:29]
h there should be a well a model called

[139:32]
models.

[139:34]
uh with the ID name

[139:40]
created up that is fine. Uh and maybe

[139:43]
uh lighter

[139:46]
ID as well

[139:49]
lighter market ID. Basically the boy

[140:03]
the

[140:05]
sorry was it

[140:09]
accounts basically this array should

[140:13]
probably be part of this thing model

[140:16]
name string uh Uh

[140:21]
API key should it be in the database?

[140:24]
Probably.

[140:26]
I don't hate that idea. Yeah, I'm going

[140:29]
to eventually we're going to replace

[140:31]
this inmemory object um with a database

[140:34]
so we can eventually add more models

[140:36]
directly to the database and it just

[140:37]
works. Um

[140:40]
[sighs]

[140:41]
models and accounts are sort of

[140:42]
synonymous here. So this API key is we

[140:46]
should write lighter API key here

[140:48]
otherwise it'll get a little confusing

[141:06]
then it would be uh probably transa not

[141:10]
transaction what would you call them

[141:12]
invocations

[141:15]
Um, every time the model is invocated,

[141:18]
we add an ID over here, model ID,

[141:22]
uh, and response.

[141:26]
And this will have a foreign key

[141:30]
relationship to

[141:32]
model. And every model will have

[141:34]
multiple uh invocations.

[141:41]
The invocation count is something we

[141:43]
should have and invocations is also

[141:45]
something we should have which should be

[141:46]
an invocations array. Uh this will be a

[141:49]
model with a foreign key relationship

[141:51]
and that looks fine. Last thing every

[141:54]
invocation will have a bunch of uh

[141:59]
we can call them tool calls. Uh so enum

[142:01]
uh tool call type would be one of two

[142:06]
create position close position and every

[142:09]
invocation will have a bunch of tool

[142:11]
calls. That's too many things here.

[142:15]
Uh tool call type invocation ID and then

[142:18]
just some meta data. That's basically

[142:22]
what

[142:23]
the model sort of gave us. Uh if the

[142:25]
model told us told us to open a position

[142:27]
at a specific uh price then that will be

[142:30]
stored in this metadata field. Um every

[142:34]
invocation will have a bunch of tool

[142:37]
calls. Every tool call will be will

[142:40]
belong to an invocation. Um and that is

[142:44]
it. There's nothing else that we need

[142:45]
over here. Um

[142:49]
I think that should do it for the

[142:50]
schema.

[142:51]
We have the models that we support. The

[142:54]
invocations um that every model has

[142:56]
done. This should definitely have a

[142:59]
created at um when was the invocation

[143:01]
started. Um

[143:03]
every invocation will have a bunch of

[143:04]
tool calls. So calls can be of one of

[143:06]
two types. Either create a position or

[143:08]
close a position. That's all that we

[143:10]
need. Other than that, we need time

[143:11]
series data. So let's do that as well.

[143:13]
Uh model uh

[143:17]
portfolio size. Um

[143:22]
for every model there will be a

[143:24]
portfolio size. um

[143:27]
which will be like a time series sort of

[143:29]
a uh a database that will have sorry

[143:31]
time series table that will have all the

[143:35]
I'll call it the

[143:38]
size is the wrong word for it probably

[143:40]
uh

[143:42]
net

[143:44]
portfolio

[143:46]
which can be an integer can be a string

[143:49]
integer is fine I guess it can't be

[143:52]
integer it needs to be a float so I'll

[143:54]
just have it as a string um created at

[143:57]
that should do it. This will be used

[144:00]
separately by a separate process that is

[144:02]
constantly calculating the price of the

[144:04]
positions um over time. How much money

[144:07]
your model has made in the last you know

[144:09]
since the beginning. Um we can use a

[144:11]
time series database over here. We don't

[144:13]
need to though uh which is why we're

[144:15]
just keeping things simple. This is

[144:16]
where the portfolio size is tracked over

[144:19]
time and everything above is just for

[144:21]
you know everything else. If you want to

[144:22]
track what invocations happened and what

[144:24]
happened during those invocations that

[144:26]
will be stored over here. Um, let's

[144:28]
start a post test table locally really

[144:31]
quickly. So,

[144:33]
Docker, I hope I have a

[144:38]
There you go.

[144:40]
It's opening for a command that let me

[144:42]
start postgress locally. While that

[144:44]
starts, let me go to the env file and

[144:47]
let me quickly change this to be

[144:52]
postgress uh postgress

[144:55]
colon

[144:59]
postgress at localhost col 5173

[145:04]
sorry 5432 my

[145:10]
uh let's do a 1x prisma uh migrate dev

[145:14]
to migrate our database. So we have all

[145:16]
the entries in the table that did not

[145:18]
happen. Missing required variable

[145:22]
uh this wrong file.

[145:27]
That looks like the right one.

[145:31]
Let's try that one more time. CA.N.

[145:35]
That looks fine to me. Let's try to

[145:38]
migrate it.

[145:42]
Uh database URL database URL postgress

[145:49]
as logos. Oh, wait. Not that this is the

[145:52]
error, but uh

[145:54]
uh

[145:56]
AI

[146:03]
missing database URL.

[146:07]
What the hell?

[146:12]
schema of Prisma

[146:21]
missing by var

[146:26]
missing something very obvious here. Uh

[146:29]
what could it be?

[146:35]
fine.

[146:38]
Probably isn't picking it up from so uh

[146:42]
export database

[146:45]
URL equal to

[146:48]
one thing I don't love about Prisma is

[146:50]
how quickly they change

[146:52]
things that they're doing. Um okay, so

[146:55]
this time it worked. Uh index of an

[146:58]
unknown field which is fine. I can get

[147:00]
rid of that field.

[147:04]
Portfolio size and portfolio [snorts]

[147:07]
model ID is good.

[147:09]
There is a

[147:12]
foreign key relationship that we need

[147:13]
other than that we're good. What's up

[147:16]
now? Portfolio size ID

[147:25]
uh

[147:31]
portfolio size ID. Every model will have

[147:35]
a one to many relationship with this

[147:36]
guy. So

[147:39]
portfolio size

[147:43]
will be a portfolio size array and

[147:48]
uh will be model that looks fine to me.

[147:51]
Now database has migrated.

[147:59]
Let's quickly add the three models in

[148:02]
here. Let's add one for now. Uh add a

[148:06]
record name is going to be Claude.

[148:10]
Lighter market ID. Lighter market ID

[148:16]
for a model

[148:19]
is not needed. Is it?

[148:24]
I don't think this is needed.

[148:29]
Yeah, that looks wrong to me. Let me

[148:31]
remigrate the database.

[148:35]
Remove unnecessary

[148:40]
field.

[148:42]
Let's do that one more time.

[148:46]
Claude model name. Claude lighter API

[148:49]
key is the one that I had hardcoded in

[148:51]
here.

[148:58]
and invocation count will be zero for

[149:00]
now and everything else is fine.

[149:03]
Why can't I save these changes?

[149:12]
Add record

[149:16]
key save.

[149:18]
There we go. That should do it. Now

[149:22]
what I'm going to do is in index.ts

[149:26]
actually in a bunch of places um but at

[149:28]
least here I should not use

[149:35]
this. Where all is this being used?

[149:39]
Um oh only one place

[149:43]
to do. Uh I'm going to get rid of this

[149:47]
and

[149:50]
do is uh let me write a function main.

[149:58]
This job is simple uh

[150:01]
con model models equal to find all the

[150:03]
models that exist

[150:06]
in the Prisma client. So

[150:12]
import

[150:13]
prisma client from dot slash oh boy

[150:19]
generated. Did I choose the wrong? Okay,

[150:21]
it's fine.

[150:23]
Generated

[150:29]
dash prisma

[150:32]
dash line.

[150:35]
There we go. launched

[150:38]
Prisma create a new Prisma client. Go

[150:40]
back to the bottom. Prisma dot

[150:42]
models.find many and then for models of

[150:46]
models uh or you know

[150:51]
models dot

[150:54]
do we going to do this? No. Um

[150:57]
model models that looks better.

[151:00]
I am going to now call uh invoke agent

[151:05]
with the model. I see no type errors

[151:07]
because the expected input here is API

[151:11]
key name. Model name, API name, model

[151:13]
name and API key. Now this is lighter

[151:15]
API key. Uh the only difference why

[151:19]
don't I see a type error here though.

[151:24]
Okay, let me just change this a little

[151:26]
bit. API key, model name, and

[151:34]
name.

[151:36]
Okay, one big thing that I messed up was

[151:39]
model name is this

[151:44]
uh open router model name. All righty.

[151:49]
Now, can I get rid of this? Is this

[151:50]
being used at all? It isn't. Get rid of

[151:53]
this. That should be good enough. Um the

[151:56]
main function should now do what it's

[151:58]
supposed to do. Um hopefully uh

[152:01]
invocation times would now become uh

[152:07]
oh it will just be invocation counter

[152:08]
right. So uh

[152:16]
yeah let's just we get it over here

[152:21]
actually.

[152:23]
Yeah, I'll just add that over here.

[152:28]
Invocation count will become part of

[152:32]
this guy right here.

[152:36]
And

[152:38]
that is exactly what we will pass over

[152:40]
here.

[152:45]
That looks good to me. Uh portfolio

[152:48]
value. This is the one thing we still

[152:50]
need to fix. Um and we'll do that soon

[152:52]
enough. This isn't part of the database.

[152:53]
This is part of the lighter SDK. So,

[152:56]
we'll get to that soon enough. Um,

[152:59]
before any of this happens, should we

[153:01]
Yeah, we should probably create con uh

[153:09]
model invocation equal to create a new

[153:12]
invocation. um

[153:16]
for a specific model ID

[153:22]
do

[153:26]
ID also needs to be sent here.

[153:33]
Create an invocation for a specific

[153:35]
model ID.

[153:37]
Hardcode the response to an empty

[153:39]
string. That is fine. Um and now

[153:44]
whenever there is a tool called to close

[153:46]
all positions [snorts] we will cancel

[153:48]
orders. Yes. And then we'll also create

[153:52]
a new tool call for this specific

[153:53]
invocation ID. Um

[153:59]
that is perfect. And whenever there's a

[154:01]
tool called to do this is then we'll

[154:04]
create a new invocation ID with symbol

[154:09]
side and quantity. Um and then finally

[154:12]
when we have the response

[154:16]
we will update the invocation

[154:20]
to update the response. Uh

[154:25]
perfect. Thank you. A and that happened

[154:27]
fairly quickly. Let me call the main

[154:29]
function. Let's see when the main

[154:32]
function is called does the agent get

[154:34]
invoked and do we log everything in the

[154:37]
database or not? Let's do a

[154:41]
for this one index.ts.

[154:46]
Oh boy.

[154:49]
Cannot find runtime/ library. Okay.

[154:53]
Yeah, I don't love Prisma anymore. Uh

[155:01]
my client

[155:06]
I hope this documentation is up to date.

[155:09]
uh

[155:17]
do/prisma/ db.

[155:28]
Uh

[155:30]
what's wrong over here? Can't find

[155:34]
1x Prisma generate.

[155:38]
client / runtime/ library from

[155:42]
some place.

[155:45]
Pretty sure this is a Google level error

[155:48]
which is what we will do.

[155:52]
And the response says

[156:10]
what the hell?

[156:12]
Uh,

[156:16]
this will not work. This is what Prisma

[156:19]
got changed to. Uh

[156:23]
but ever since then it's not working. Uh

[156:26]
okay, there we go. Dot slash Prisma

[156:29]
client from Oh, you don't need this for

[156:34]
Postgress though, do you? Uh

[156:39]
Pisma Postgress one.

[156:49]
This is what scares me. Uh, this is

[156:52]
something that you didn't need to do

[156:54]
before, but seems like you need to do

[156:57]
now. Um,

[157:00]
let's see.

[157:03]
Maybe this was it. Yeah, this is it. All

[157:06]
righty.

[157:10]
Wait for it.

[157:12]
Do we have a long? Do we have a short?

[157:14]
What do we have, sir?

[157:22]
something we have. We I removed all the

[157:24]
locks. I don't know what happened. Um

[157:26]
seems like it is a I don't know. We'll

[157:29]
see. It is a long that we created on

[157:32]
Ethereum. A $500 value uh valued

[157:37]
transaction. Great. Uh now comes the

[157:40]
bigger question. Was any of this logged

[157:43]
to the database? Um and hopefully it

[157:45]
was. Look at this guy. And if we open

[157:48]
the invocations, we have one invocation

[157:52]
which returns everything. We've logged

[157:53]
it over here. Beautiful. It had one tool

[157:56]
call which is great. It also looks

[157:58]
something like this. It had everything.

[157:59]
Um we have everything we need folks.

[158:02]
Everything is being logged into the

[158:03]
database. What's left is filling the

[158:05]
portfolio for which we need to

[158:06]
understand how do you grab your

[158:08]
portfolio from this guy. Um oh well.

[158:12]
Haha. The good thing is we have

[158:16]
a button we can click and then inspect

[158:20]
bunch of things over here.

[158:22]
What the hell? How do I not have $3,000

[158:26]
to begin with? Oh, because somebody

[158:28]
else. Never mind. Um, anyways, what I

[158:31]
was saying was P&L my index public pools

[158:34]
portfolio

[158:37]
seems like this is the one. No, this

[158:39]
isn't it. Uh let me clear everything.

[158:43]
Let me

[158:47]
go back to trade

[158:50]
then come back to portfolio. And do we

[158:52]
see anything?

[158:54]
I see public pool metadata.

[158:58]
This is nothing. PNL

[159:02]
trade PNL time stamps. It still doesn't

[159:04]
give me my uh

[159:08]
portfolio.

[159:10]
It returns me various P&Ls which is what

[159:13]
I don't need right now. I need my

[159:15]
portfolio. I just need this number.

[159:17]
Trade equity is also good enough. Um if

[159:19]
I get this I will know the current

[159:21]
balance. Question is where does it come

[159:23]
from? Probably comes from here only. Uh,

[159:29]
nope.

[159:33]
This isn't it.

[159:35]
This is

[159:38]
refresh the portfolio page probably. Is

[159:43]
this only then accounts

[159:47]
index?

[159:51]
No.

[159:54]
Four accounts. Beautiful available

[159:57]
balance. Haha, that sounds easy enough.

[160:00]
Um,

[160:01]
yeah, that wasn't beautiful. Uh, okay.

[160:05]
Okay. Okay. One more thing we need. Uh,

[160:09]
index is there.

[160:13]
Account type 011

[160:19]
index and then index are crazy. After

[160:22]
this, this is the index for this guy.

[160:24]
This is the account index of this guy,

[160:26]
which is weird, but we don't really

[160:29]
care. Uh, okay, sounds good. So, what we

[160:31]
need now is each one of these will also

[160:33]
have an account index. Um,

[160:38]
what was the field again? Available

[160:39]
balance. Does this guy have an balance?

[160:41]
Perfect. So, one more thing we need guys

[160:44]
is the following. We need another field

[160:47]
in the Prisma. schema which will be

[160:49]
called the account index. So every model

[160:54]
will have an account index uh which will

[160:57]
be an integer is fine. Can also be a

[161:00]
string. It's a pretty big integer. So

[161:02]
I'd probably just stick to string for

[161:03]
now. And it'll I'll also like I would

[161:06]
like to replace the

[161:09]
model name with open router model name.

[161:15]
Um so it's just easy to understand.

[161:17]
Yeah. If I open models.

[161:21]
Yeah, model name needs to be open model

[161:23]
name. It's a better name for it. Let me

[161:25]
copy this and keep this handy.

[161:29]
Uh, let me migrate my database one more

[161:31]
time.

[161:37]
Uh,

[161:40]
oh,

[161:42]
no default needed

[161:45]
for this guy.

[161:56]
>> [snorts]

[161:56]
>> Okay, what I'm going to do is get rid of

[161:58]
this guy.

[162:00]
It shouldn't be very easy because I need

[162:02]
to first get rid of this guy. Oh, I

[162:04]
don't have the studio open.

[162:08]
Let me open it. Dismiss.

[162:12]
Open this guy.

[162:15]
What the hell?

[162:24]
Uh yeah. Okay. Okay. Okay. Um

[162:28]
let me get rid of uh

[162:35]
this and

[162:39]
now let's do it.

[162:42]
Okay. Delete.

[162:45]
Yeah, it won't delete because forgot.

[162:51]
I have to first delete the tool calls,

[162:57]
then delete the invocations,

[163:02]
then delete the model.

[163:06]
Uh,

[163:11]
then migrate the database again.

[163:19]
Then open the studio again. Now read the

[163:22]
modules again. So

[163:25]
uh add record.

[163:29]
Oh wait

[163:31]
add these two fields. Remigrate the

[163:34]
database.

[163:41]
uh re studio followed by finally

[163:46]
uh discard changes add in your

[163:51]
something is up here I don't see the

[163:53]
model name anymore

[164:00]
singles

[164:06]
bro the hell okay and that and that

[164:09]
there we go and a record going to call

[164:12]
it claude. This was this lighter API key

[164:16]
is this

[164:25]
immigration counter zero account index

[164:28]
beautiful is what is the clawed account

[164:31]
index? Um

[164:33]
let's just use this one for now. Um

[164:39]
we can change that eventually.

[164:42]
Good enough. Uh now comes the question

[164:44]
of getting the portfolio balance which

[164:46]
as we saw was as simple as hitting

[164:50]
um [clears throat] oh boy.

[164:58]
So public endpoint.

[165:01]
Yeah it is. Okay. Uh that is fine. We

[165:05]
have another thing to add folks. Um,

[165:09]
this is going to be L1 address. This is

[165:12]
my it's basically my layer 1 blockchain

[165:15]
address. Uh, it's basically my primary

[165:17]
address. Whenever the blockchain becomes

[165:18]
a little more open, you can explore them

[165:20]
on the explorer. You can explore my

[165:21]
account over here. Why we need this is

[165:23]
because this is good enough or all that

[165:26]
you need to get the portfolio of a user

[165:28]
by hitting this end point. So, I can

[165:29]
simply hit this endpoint. uh uh create a

[165:32]
new file probably get

[165:37]
excuse me I'll call it uh

[165:41]
get portfolio is what I'll name it get

[165:43]
portfolios

[165:56]
uh

[166:02]
Yeah, every account

[166:08]
will now also have a one.

[166:12]
Yeah, we'll call it an account. Next um

[166:18]
con

[166:20]
response

[166:23]
equal to a wait

[166:26]
access get.

[166:39]
Oh wait,

[166:42]
actually you can do this also by account

[166:46]
index

[166:47]
and value would be

[166:51]
um

[166:54]
three would be wait for it

[166:58]
let's say I do this

[167:03]
oh index my

[167:12]
Okay.

[167:13]
So, yeah, we don't need the fancy

[167:16]
looking L1 address anymore. Um, it'll be

[167:20]
as simple as doing this. Uh,

[167:27]
perfect. and

[167:31]
taxius and return accounts of zero dot

[167:38]
available balances. Uh actually balance

[167:41]
is not the right thing. Uh

[167:45]
yeah, unfortunately available balance is

[167:47]
not the right thing. Uh or is it?

[167:51]
No, actually we'll see. Uh give me a

[167:53]
second here.

[167:56]
Oh boy, this is going to get really hard

[167:57]
now because I need to reauthenticate I

[167:59]
think. Oh no, I don't. Uh, this is my

[168:02]
net equity. Oh boy, this is across

[168:07]
all accounts which is a little scary

[168:09]
now. Oh no, this is not a across all. My

[168:11]
bad. This is the main account. Let me

[168:13]
open an order. Um, let me place you know

[168:16]
a fairly small position um

[168:19]
to let's say long. Place market order

[168:22]
portfolio. So now if I look at this,

[168:25]
there you go. $100 defense available

[168:27]
balance of total equity. Question is uh

[168:33]
what was the ID of this guy?

[168:39]
Headers and

[168:41]
this one right here. And where do I see

[168:46]
$2,900? Uh do I see them in available

[168:49]
balance? Probably not. Yeah, that's

[168:52]
2,800. Collateral is the one that we're

[168:54]
looking for.

[168:56]
So, dot we don't need any of that.

[168:59]
Accounts of zero

[169:03]
of zero dot collateral. That's good

[169:05]
enough.

[169:06]
Yeah, we're good. Uh,

[169:10]
perfect. That returns a string only.

[169:16]
And we're good. That's done. Let me

[169:18]
close the position that I had opened a

[169:20]
while back.

[169:23]
Boy, I'm down $3.

[169:27]
$4. I should have asked the guy. All

[169:29]
right, that's done. Close. Close. Uh,

[169:33]
now comes the best part.

[169:36]
Them can now actually know the

[169:40]
current balance. Uh so cost

[169:46]
perfect portfolio equal to get portfolio

[169:49]
account and this portfolio is what we're

[169:52]
going to forward

[169:54]
it's through the LLM. Um then we're good

[169:57]
to go. Are we think we're good to go. Um

[170:02]
the one thing that we need to add now is

[170:03]
the tracker that will actually track the

[170:05]
time series data um over time. I'll add

[170:09]
that as well very soon. Um right after

[170:11]
not right after let's just complete it.

[170:12]
Uh let's create a new file here called

[170:17]
price tracker.ts

[170:21]
which is basically going to be a set

[170:23]
interval and it's going to find all the

[170:26]
models.

[170:28]
Uh

[170:34]
yeah this is perfect. part. This needs

[170:37]
to change a little

[170:40]
fine.

[170:42]
Dot at create

[170:48]
data, model ID, net portfolio. This is

[170:49]
perfect. And then time automatically.

[170:52]
Yeah, this needs to run every

[170:55]
5 minutes is fine. I guess every 2

[170:56]
minutes is also good. Every 2 minutes,

[170:58]
we'll run this to get the net portfolio

[171:01]
of every model. Um, perfect. I don't

[171:04]
think there's anything wrong here. Um,

[171:06]
let me try to Should I run it? Yeah,

[171:09]
let's run it. Bun index dot bun price

[171:12]
tracker.ts.

[171:16]
Uh,

[171:18]
it'll take 2 minutes to start. Let me

[171:19]
just change this to be 1 second for now.

[171:25]
All right, that should have probably put

[171:26]
my portfolio in the database

[171:29]
hopefully. Um, let's see. Let's do a

[171:35]
studio.

[171:38]
Let's look at the price track of the

[171:41]
portfolio size. And it looks something

[171:42]
like this over time. Beautiful. Exactly

[171:45]
what we needed.

[171:47]
What is left? Nothing. Nothing is left,

[171:49]
folks. Everything is done. We have I

[171:52]
just need to productionize this, which

[171:54]
means I need to number I mean, let's get

[171:57]
into that section next. Everything is

[171:58]
done. Um we have

[172:01]
bunch of files here but if you really

[172:02]
look at it we have two primary files.

[172:04]
One is this price tracker whose job is

[172:06]
simple. Just get the current portfolio

[172:08]
and put it in the database so that

[172:09]
eventually uh we can show the time

[172:12]
series data u something like this in a

[172:16]
graph. Um that's the job of this price

[172:19]
tracker. The job of index.ts Yes. Um is

[172:22]
to call this invoke agent function every

[172:28]
uh what is the new field that we added

[172:30]
index

[172:36]
and

[172:38]
perfect. Um this main function's job is

[172:41]
to um

[172:43]
idly call this every few whatever

[172:49]
2 minutes or something like that or

[172:51]
every probably every 5 minutes

[172:56]
every 5 minutes call the main function

[172:58]
which is going to for every model that

[172:59]
exists in our database invoke the agent

[173:01]
and the agent would tell what to do and

[173:02]
it will do it. Um I would like to see

[173:05]
just once what prompt we're actually

[173:07]
sending in the end. So this very big

[173:09]
thing over here

[173:14]
enriched prompt equal this thing

[173:19]
going to be the enriched prompt

[173:26]
and

[173:32]
also to log this. So

[173:36]
logage

[173:38]
prompt let's see I'm just curious to see

[173:41]
the prompt that we have written until

[173:42]
this point and

[173:46]
that's there that's there's probably

[173:47]
scope of improvement there before I

[173:48]
start running this today I want to make

[173:50]
sure the prompt is at least decent

[173:52]
enough u oh boy again the same thing

[174:02]
we go that's the prompt. Um, you are an

[174:04]
expert trader. You were given $50 to

[174:07]
trade. So, I'm going to change that

[174:15]
trade with you think trading on crypto

[174:17]
marketation.

[174:19]
Uh

[174:25]
current uh

[174:27]
ETH BTC 00 long uh your current

[174:30]
portfolio

[174:31]
value is this thing. Okay. Finance

[174:34]
information

[174:36]
intraday long-term. What the [ __ ]

[174:40]
Candles are not enough, sir. We're not

[174:42]
sending enough data. We're only sending

[174:43]
the candles. U there are so many nice

[174:46]
indicators that we wrote. I'm going to

[174:49]
add them really quickly. Prompt

[174:51]
portfolio size in day positions is mid

[174:54]
prices. So, it's going to be a little

[174:56]
more complicated than this. So, uh mid

[175:00]
prices this thing

[175:03]
and then uh

[175:07]
RSI and what is the other thing? No, not

[175:09]
the RSI. We calculated the EMA 20 is

[175:26]
dot uh in 20s dot join

[175:32]
and then MACDs

[175:37]
equal to

[175:38]
inter indicators

[175:41]
dot MD dot join

[175:45]
That looks much better um for the inter

[175:48]
position. And the same thing used to

[175:49]
happen for this guy. Um

[175:52]
something happened to my AI. It's not

[175:54]
helping me anymore. Um

[175:57]
take a break. Oh, there you go.

[175:58]
Something came and then went away.

[176:02]
What happened to cursor? Why is it not

[176:04]
performing?

[176:12]
All right.

[176:14]
That looks good. Much better hopefully.

[176:15]
Let's run that one more time.

[176:18]
Okay, there we go. Oh boy. EMA 20 is uh

[176:23]
a lot of decimals.

[176:26]
Probably doesn't need to be these many

[176:28]
decibels. Uh long-term and then

[176:30]
intraday. Everything is fine. The only

[176:33]
problem is precision we don't need. So,

[176:36]
I'm going to go back

[176:38]
to um

[176:43]
get indicators and

[176:46]
do something here. Uh

[176:49]
map

[176:55]
is good.

[177:07]
Basically the same thing on all three.

[177:12]
Let's try that one more time.

[177:16]
Uh

[177:18]
mid prices, EMA 20s, MACDs for the

[177:21]
long-term position, for the short-term

[177:22]
position. Uh

[177:26]
you have a crypto.

[177:29]
All right, that is perfect. Uh

[177:31]
what were our tools called though? Were

[177:34]
they called

[177:37]
create position

[177:41]
and

[177:43]
close all position? My bad.

[177:47]
So prompts.

[177:59]
Okay, that looks better. Um, let's see

[178:02]
what their prompt was. It probably has a

[178:03]
few things that we should add as well.

[178:05]
Um,

[178:10]
this we forgot. We should probably add

[178:11]
that. Um,

[178:15]
this we should add

[178:24]
uh

[178:29]
me candles that's there

[178:35]
five minute

[178:39]
intraday candles

[178:42]
latest

[178:45]
and same thing over here.

[178:48]
What else? Uh oh boy.

[178:55]
Yeah, boy.

[178:58]
We need to add positions for a bunch of

[179:00]
markets and not just um

[179:04]
one market, which is what we're doing

[179:05]
right now. That side was only placing

[179:08]
shots on Ethereum because we were only

[179:09]
giving it Ethereum data. Um but we need

[179:12]
to give it data for all the markets, not

[179:14]
just Ethereum. Uh get indicators. It

[179:17]
does take market input. So why was I an

[179:20]
idiot and not giving it all the market

[179:23]
IDs? Um

[179:26]
yeah uh so in our constants somewhere

[179:30]
maybe in markets.ts

[179:33]
we have this. So for every market we

[179:35]
need to get this uh

[179:39]
cursor tab desperately need it

[179:43]
to be a little fast right now but it

[179:45]
won't work. Um const

[179:49]
equal to um

[179:52]
markets do map. Oh, sorry. Oh boy.

[179:55]
Object dot keys of markets do map

[180:01]
market uh slug.

[180:08]
uh

[180:12]
markets of market slug dot

[180:17]
market ID

[180:21]
and the same thing over here

[180:30]
return

[180:32]
indicators

[180:35]
longterm indicators this. All righty.

[180:39]
That looks Oh boy, not great to me. Um

[180:46]
this will be

[180:48]
sync

[180:50]
and this will be await promise do all

[180:57]
and now

[181:02]
this would be a lot of things. Um,

[181:16]
internet positions.

[181:18]
Uh, okay. This is going to be really

[181:20]
long now. Um,

[181:22]
which I don't love. But there's not much

[181:24]
I can do. Is there prom.ts will have to

[181:28]
change. Basically this there's no easy

[181:30]
way to

[181:32]
change the

[181:34]
prompt um

[181:38]
like this. So what I'll have to do is

[181:40]
replace this with uh

[181:43]
all indicator

[181:46]
data. Um let's just create that over

[181:49]
here. up.

[181:52]
Let's get rid of this dot replace

[181:57]
all underscore

[182:00]
indicator

[182:04]
data uh with a very complicated string.

[182:07]
now um which I'm going to call

[182:10]
all_indicator

[182:14]
data um that's what I'm going to create

[182:17]
over here right after this const all

[182:20]
indicator data equal to uh now it gets u

[182:25]
a little challenging um let's say empty

[182:28]
string um indicators dot for each um

[182:34]
uh I

[182:36]
Why can't I simply do this?

[182:43]
Uh indicator data market. Yeah.

[182:48]
Uh all indicated data equal to uh all

[182:53]
data plus

[183:00]
uh

[183:08]
this would be

[183:11]
intraday indicators. Uh so

[183:14]
mid prices

[183:21]
dot

[183:24]
prices dot join

[183:28]
comma and then uh

[183:32]
EMA 20

[183:37]
and then MACD

[183:44]
then for the long term it'll be very

[183:46]
similar

[183:50]
Aha, cursor is back. All indicated data

[183:54]
equal to all indicated data plus this

[183:56]
thing. Beautiful. Um, and we should also

[183:59]
put a very big [snorts]

[184:01]
market name over here. Market is

[184:05]
market slug. Yeah, that's perfect.

[184:10]
Now, let's look at the prompt. Now,

[184:11]
we'll have a much better, more relevant

[184:14]
prompt. U let's see there we go um

[184:20]
your all of this is latest um market BTC

[184:25]
this is the intraday candle this is the

[184:28]
long-term candle market soul this is the

[184:30]
intraday candle that looks so wrong this

[184:32]
is not soul prices u this is also ETHM

[184:36]
prices this is also Ethereum prices this

[184:38]
alothe I missed what I have missed is

[184:42]
that when I call this I Call with the

[184:45]
market ID.

[184:48]
Get indicators.

[184:50]
Are you expecting this market ID at all,

[184:52]
sir? You are not. Well, you should. Um,

[184:56]
sle market ID needs to be replaced with

[184:57]
this market ID and we need to get rid of

[184:59]
this variable.

[185:01]
Now, let's try it one last time. Pretty

[185:04]
sure it'll work this time. Let's look at

[185:06]
the prompt. You're an expert trader,

[185:07]
yada yada yada. Ethereum prices look

[185:09]
fine to me

[185:11]
right here. Soul prices look fine to me

[185:14]
right here. BTC prices uh look fine to

[185:18]
me right here. Um if I quickly compare

[185:21]
the MACD on these guys, let's see what

[185:25]
these guys have for a slightly more

[185:28]
long-term position on let's say Solana.

[185:32]
Their long-term MACD looks something

[185:34]
like this. 3.169 3.088. What does ours

[185:38]
looks like?

[185:40]
3.45 four five 3.3 33 33 33 three three

[185:43]
and then slowly averaging towards a two

[185:46]
and a one for these guys starts from

[185:48]
three slowly goes to two slowly goes to

[185:49]
one so looks pretty close um perfect

[185:52]
prompt looks good to me now uh what else

[185:55]
do they have at the end after they give

[185:57]
all the data do they give anything else

[185:58]
in the prompt um here's your account

[186:01]
information and performance that is

[186:02]
something they give uh which we don't

[186:04]
yet um probably something to add um

[186:09]
total return available able cache,

[186:12]
current account value, live position and

[186:15]
performances. We should probably give

[186:17]
that as well. Uh yeah, let's add that

[186:20]
really quickly. Um here is your current

[186:24]
performance in prompt.ts.

[186:29]
Here is your current performance.

[186:36]
And I should probably put all of this

[186:45]
Um

[186:49]
available cache

[186:54]
uh something we'll have to add now

[186:58]
available

[187:01]
cache

[187:02]
and then uh what else do you have

[187:06]
current account value

[187:09]
I've given it once I'm going to give it

[187:11]
one more time

[187:13]
portfolio value or current account value

[187:17]
and then lastly the open positions right

[187:20]
current

[187:22]
positions and performance

[187:30]
current account positions. All right,

[187:34]
let's quickly add those uh

[187:37]
as well. So

[187:41]
amongst the many things we replace here,

[187:42]
I'm going to also add dotreplace

[187:46]
available cash with

[187:51]
something. I'll tell you what that is.

[187:52]
Um and then

[187:56]
current account value which is just the

[188:02]
dollar followed by the portfolio

[188:05]
and then

[188:07]
the last thing which is the

[188:18]
uh current account positions. Um

[188:23]
firstly, yeah, there you go. That's

[188:25]
right here. So, it's first part. Um what

[188:27]
is it? Yeah, I think it is. Is it? No,

[188:28]
it isn't. Problem is, oh yeah, we have

[188:31]
everything. Liquidation price,

[188:32]
unrealized PNL realized PNL. So, there

[188:35]
you go. Um

[188:40]
yeah, that's fine. Just JSON stringify

[188:43]
it. And lastly, available cache um is

[188:47]
something we will get from I'll tell you

[188:50]
where this guy you should not return a

[188:53]
string generally.

[188:56]
We should have returned two things. Um

[189:00]
total this thing and available string.

[189:03]
Um and yeah, we should have returned

[189:06]
total this thing and then available

[189:10]
would be wait for it.

[189:12]
uh if I have that endpoint open

[189:14]
somewhere. I don't do I don't or do I do

[189:17]
here if you look at we have collateral

[189:20]
and then we have

[189:23]
uh come on available by so

[189:27]
this thing right here uh response

[189:30]
oh dot data accounts

[189:34]
of zero dot variables uh

[189:38]
this needs to be response so does this

[189:41]
guy and wherever this is being used

[189:45]
which was in the price tracker. So let's

[189:47]
fix that.

[189:49]
portfolio

[189:51]
dot total.

[189:53]
And here

[190:00]
portfolio

[190:04]
dot total

[190:07]
dot total

[190:13]
dot available any any place else where

[190:16]
portfolio is being used. I don't think

[190:18]
so.

[190:20]
Yeah, we're fine. All righty. Um,

[190:24]
looking at the prompt one more time, I

[190:25]
think we should be good this time. Um,

[190:27]
yeah, there you go. Current account

[190:29]
value is this. Uh, available cash is

[190:32]
this. Oh, current account value looks

[190:34]
wrong. 2982.

[190:37]
Uh, so that looks right. Yeah, it's

[190:39]
fine. Um, and uh, current live positions

[190:43]
symbol e position un 0. Everything is

[190:45]
zero. Let me change that a little. Uh

[190:47]
let me create a small long position

[190:50]
and let's see

[190:53]
do we now get a different available and

[190:56]
total cash. So available cash is only

[190:58]
this much total cash is this much.

[191:00]
Performance also tells there is a long

[191:02]
position that is open with an unrealized

[191:03]
P&L of plus 0.89 because it's in the

[191:06]
positive. If it was in the negative it

[191:08]
would have shown that as well.

[191:10]
Beautiful. I think we are good to go

[191:12]
guys. Um what I'm going to do now is

[191:15]
blockize this. Um I basically want to

[191:17]
run this all night so that by tomorrow

[191:19]
we have some data. To productionize this

[191:21]
I have to do a few things. The very

[191:22]
first thing I have to do is keep my

[191:24]
laptop open all night when it runs. Um

[191:26]
eventually I'll prodize this on a cloud

[191:28]
server. Right now I'll just run it on

[191:29]
the laptop through the night. So by

[191:31]
tomorrow we have some data some graphs

[191:33]
to look at. Um

[191:36]
the other thing I'm going to do is uh we

[191:39]
should have added a bunch of try catches

[191:40]
here pretty surely rat fatigga and then

[191:42]
when it does break uh you know I will

[191:46]
have a bunch of open positions and no

[191:47]
one rebalancing them. Um but that is

[191:50]
fine. Let me quickly go back to uh tools

[191:54]
followed by uh sub accounts and transfer

[191:58]
$1,000

[192:00]
to all three of these. So, uh, how do I

[192:03]
transfer money from here to here though?

[192:05]
Pretty sure I did that a while back. Uh,

[192:11]
I will need to connect my ledger

[192:16]
and then

[192:25]
uh I forgot transfer.

[192:30]
There we go. Uh, from main account to

[192:33]
claude, $1,000

[192:36]
transfer

[192:50]
from main account to Quen.

[192:54]
$1,000

[192:56]
transfer.

[193:05]
Um,

[193:07]
from main account to

[193:13]
deepse is the one that's left, I think.

[193:15]
Deepseek. There we go. It has $49 in it.

[193:19]
So, it needs 951

[193:21]
or something like that. Transfer.

[193:32]
All right, that is good. Now I'm going

[193:34]
to create a bunch of API keys u for

[193:36]
these three sub accounts. I'm going to

[193:38]
hide that from you guys. Um I'm also

[193:40]
going to delete the API key I created

[193:41]
for the main account. So if I go to API

[193:43]
keys and number two, I'm going to

[193:48]
basically get rid of the one that I

[193:50]
already had. Um,

[193:53]
and then let me hide this really

[193:55]
quickly.

[193:57]
Uh, how do I hide this? Okay, wait.

[194:01]
There you go.

[194:08]
Perfect. That's done. Um,

[194:11]
okay. I got rid of

[194:14]
the API key that I created in front of

[194:16]
you guys, so you guys can't see it

[194:17]
anymore. And now I need to quickly

[194:19]
create three separate API keys. um for

[194:21]
the three separate sub accounts that I

[194:23]
have. I'm going to do that offscreen. Um

[194:26]
and once that is done, I'll then take

[194:28]
you through the process of how I seeded

[194:30]
the database and started the process. Um

[194:32]
so let me quickly do that and I'll see

[194:34]
you guys in 2 minutes. All right, I have

[194:37]
created all the private keys that I

[194:39]
need. Um I have created a Neon DB post

[194:42]
database on the cloud. I've added all

[194:43]
the three models over here. Now

[194:46]
hopefully when I run the script uh it

[194:48]
just works for all three models uh and

[194:51]
starts to place orders. Uh I'm going to

[194:54]
do this on a digital ocean uh draw plate

[194:57]
so that I don't have to run this uh you

[195:00]
know locally myself. U if my Mac crashes

[195:03]
it still works. So let me log into one

[195:07]
of

[195:10]
this yeah one of the digion servers that

[195:14]
I have where I'm already running a

[195:16]
market maker sort of a bot. Um

[195:21]
let me I need to copy this code over

[195:24]
here now. I think it's safe enough to

[195:27]
open source this. Uh so let me quickly

[195:30]
do that.

[195:32]
Let me add a new repo here called AI

[195:37]
trading agent.

[195:43]
Let me publish.

[195:47]
One thing I should do is

[195:51]
remove the

[195:55]
git folder in the lighter SDK. Uh

[195:59]
otherwise it'll just be a subm module.

[196:00]
will complicate things. Get init.

[196:04]
Get add dot. Have I added? Do I have

[196:06]
something scary in here? I don't think

[196:08]
so.

[196:11]
Status.

[196:13]
That looks okay to me. I probably have

[196:17]
something in there I shouldn't, but I'm

[196:19]
just going to commit and push. I don't

[196:21]
have any of the wrong API keys or

[196:22]
private keys, which is the only thing

[196:24]
I'm worried about. uh in it followed by

[196:28]
a

[196:29]
get remote

[196:32]
at origin this thing get push origin

[196:34]
head.

[196:39]
Now let me clone this repo

[196:44]
on the digital ocean box that I have.

[196:58]
Let me do uh AI trading agent uninstall.

[197:05]
CD lighter SDK uninstall

[197:09]
CD dot dot v.NV.

[197:14]
Uh now I'm going to hide a few things

[197:17]
from FD1. I'm basically

[197:20]
adding the Postgress database URL um

[197:24]
on the server.

[197:28]
And I'm going to get rid of the open

[197:30]
router API key that I showed you guys

[197:32]
and I'm going to create a new one um

[197:35]
that I'm going to use on the production

[197:36]
server. So let me do that really quickly

[197:55]
API keys. Deleting the one that you guys

[197:58]
saw.

[198:00]
creating my own fresh API key that is

[198:04]
not shared with the world

[198:06]
called digital ocean server AI trading

[198:10]
agent.

[198:12]
Copy it and replace it in the env file.

[198:18]
Uh

[198:27]
and I think that should do it. Uh let's

[198:29]
do a bunx Prisma generate to generate

[198:34]
the [snorts] Prisma client. And I see an

[198:36]
error. I think I can show this to you

[198:37]
guys now. Uh

[198:41]
the error is again missing environment

[198:43]
variable database URL. I don't know why

[198:45]
it makes you export the environment

[198:48]
variable. Why does it not pick it up

[198:50]
automatically from

[198:53]
uh the ENV file? I think this will be

[198:55]
problematic later also when I run the

[198:57]
process. But for now, I'm just going to

[198:59]
export that the database URL as an

[199:02]
environment variable. Now I'm going to

[199:04]
run bun

[199:08]
express generate to generate the client.

[199:12]
Seems to have worked. Uh bun wait for

[199:16]
it. It was called

[199:20]
I forgot index.ts only, right? Bun

[199:23]
index.ts can't find axios. One add axio

[199:27]
one index ts.

[199:30]
All right. Moment of truths.

[199:38]
The very first call went

[199:41]
for the first model I would think. Yeah.

[199:47]
And god knows what happened here.

[199:50]
A very first error came which is a 400.

[199:54]
and it says

[199:56]
invalid signature.

[199:58]
This is going to get really hard to

[200:00]
debug, guys.

[200:03]
Let's first see do we have any open

[200:06]
orders in our lighter trading engine?

[200:14]
Do we have any open orders? Not for sub

[200:16]
account one. Let me go and switch my sub

[200:19]
account to sub account two. Do we have

[200:23]
any open orders here? We don't. Let me

[200:26]
switch to sub account three. Do we have

[200:29]
any open orders here? We don't. We don't

[200:30]
have any open orders. We got a 400 um

[200:34]
status code. Um which said

[200:39]
bad request.

[200:41]
Probably this one right here. I do see a

[200:43]
signature that looks okay to me.

[200:47]
might be that I've pasted the API keys

[200:51]
incorrectly. Let me quickly check that.

[200:53]
I might have an extra space or something

[200:55]
in there. That's my only guess. Um, so

[200:57]
if I go to my Neon DB dashboard and I

[201:00]
open lighter API key, first one looks

[201:03]
fine. Second one looks fine.

[201:07]
All three of them look fine honestly.

[201:08]
Uh, let me rerun the process once

[201:12]
you guys can see it as well.

[201:19]
What model is this? How would I know?

[201:27]
C E3 CPTC.

[201:31]
Same error, which is fine. Same error

[201:33]
for the second guy. Probably a same

[201:35]
error for the third guy.

[201:38]
Let's see. This one's taking too long.

[201:40]
All three of them get a 400 status code.

[201:44]
I have a guess as to what Yeah, you guys

[201:46]
can see my database URL, can't you? Back

[201:48]
in a second. All right, I am back. I

[201:51]
think my file had a claude underscore

[201:56]
something private key API key. And I

[201:59]
have a feeling

[202:01]
there we go. When we create an order, we

[202:03]
use this. Why? Oh, no.

[202:06]
We don't use this, do we?

[202:09]
As an example. So

[202:11]
here we don't uh

[202:14]
to move off screen for a second. Open my

[202:16]
TMV file and search through the codebase

[202:19]
for API key cloud. Yeah, we're not using

[202:21]
it anywhere. Um

[202:24]
using it one place but that's

[202:25]
unnecessary. Okay, let me run the same

[202:28]
code locally once. Shouldn't change

[202:30]
anything. I think we have some debugging

[202:32]
ahead of us. Um let me run the same code

[202:35]
base locally. So one Hey, did you guys

[202:39]
see anything again? Maybe you did. I

[202:42]
will change the password in case you

[202:44]
did. One index.ts gives us probably the

[202:49]
same error.

[202:51]
Yeah, even worse error this time. Uh

[202:56]
a 401.

[203:02]
Oh,

[203:04]
I changed the

[203:08]
open router API key. So, let me quickly

[203:12]
change that locally as well. I expired

[203:15]
the old one that you guys saw. Uh,

[203:22]
all right, that is done. Let's try that

[203:24]
one more time. One index.ts.

[203:30]
And I'm expecting the same error

[203:32]
locally.

[203:36]
400 invalid signature which is if I

[203:40]
don't see it locally then I'm just using

[203:42]
the wrong digital ocean machine because

[203:44]
there is some

[203:46]
I do see the same error though. Um

[203:49]
invalid signature. How is this working

[203:50]
until this point? Um

[203:56]
how was this working until this point? I

[203:58]
have no idea. API key. Oh, there you go.

[204:01]
Account index is incorrect, sir. Uh, but

[204:03]
that shouldn't change anything here,

[204:05]
should it? Um,

[204:08]
account index definitely is incorrect.

[204:17]
It'll be something else. Where have I

[204:20]
hardcoded this? Here.

[204:22]
Here.

[204:24]
This looks like

[204:27]
an unused file, but this thing.

[204:30]
Yeah. Create position. Yeah. Account

[204:32]
index is going to be

[204:35]
account index.

[204:39]
Convert that to a number, please. Thank

[204:41]
you. Where else is this being used? Um

[204:45]
here.

[204:47]
So again, same thing.

[204:50]
account dot account index

[204:52]
convert it to a number

[205:01]
index and then oh oopsy daisy that was

[205:04]
incorrect

[205:06]
API key index

[205:10]
account index is what I'm looking for

[205:12]
create position does imported but it is

[205:15]
unused

[205:17]
and then Open position

[205:24]
uh get open positions should also yeah

[205:31]
have an account index here.

[205:35]
Oh god, that also means wherever this is

[205:37]
being used, we need to pass the account

[205:40]
index.

[205:48]
Same thing over here.

[205:52]
Need to pass the count index

[205:56]
and we're good to go. Uh let's see if

[205:59]
this fixed it. Can get rid of

[206:03]
this guy now.

[206:08]
So I was using the main accounts account

[206:10]
index everywhere. Um when ideally I need

[206:13]
to use the sub accounts account index

[206:15]
everywhere which is already part of the

[206:16]
database. I just forgot to put it um in

[206:19]
the code. Hopefully that's it. That

[206:22]
seems fine to me.

[206:25]
Oh boy. We've opened three positions

[206:27]
now.

[206:31]
When I say three positions, I mean we

[206:33]
will very soon open one position for

[206:35]
each of the sub accounts. That was the

[206:36]
issue. Um

[206:39]
let's see last one.

[206:43]
Let's finish this up.

[206:46]
I would love to see what's the

[206:47]
difference between all three of them. Uh

[206:49]
between what Claude did, what DeepS did,

[206:52]
and what um the third one, Quen. I think

[206:55]
this is the slowest one. This takes a

[206:57]
long time. There we go. None. All right.

[207:00]
Let's see what happened. Um, if I go

[207:03]
back here to my

[207:07]
lighter dashboard,

[207:09]
there is a 2.5

[207:11]
long ETH position opened by the first

[207:14]
guy. I'm just closing it for now. 2.5

[207:17]
long ETH

[207:19]
by

[207:22]
um Claude. If you look at Deepseek, it

[207:26]
did a 0.25 long ETH. a slightly more

[207:30]
conservative position. Why is everyone

[207:32]
only trading with ETH? Am I missing

[207:33]
something? Um why why did neither of

[207:37]
these two create a position using u

[207:44]
BTC? Well, this guy did and it's down $3

[207:46]
already. All right. So, we know that all

[207:50]
the models are interacting with and

[207:53]
logging, shorting, ETH, BTC, whatever.

[207:55]
Did I see a 20x position? Great history.

[207:59]
Or did I see a 10x leverage? Uh

[208:04]
I saw 20x for a second here. Um anyways,

[208:07]
cool. Um it is working.

[208:11]
Let me push this to the server and keep

[208:13]
the server on for the night and tomorrow

[208:16]
we'll see what happens. Uh cursor get

[208:18]
status. Get add dot getit

[208:23]
get status. Dmit

[208:28]
fixed account index bug.

[208:37]
Push the code base.

[208:44]
get pull origin

[208:50]
master.

[208:53]
Uh

[208:54]
now I need to run two processes. Uh

[208:59]
there is a PM2.

[209:05]
There you go.

[209:07]
PM2 start interpreter

[209:11]
1 index.ts.

[209:14]
And then PM2 start interpreter one. Uh

[209:18]
what was the other one called? The one

[209:20]
that would

[209:23]
uh

[209:28]
price track I think it's price

[209:29]
tracker.ts.

[209:32]
All right. The first one itself errored.

[209:34]
So men logs one

[209:40]
sorry PM2 logs one

[209:44]
PM2 stop one uh

[209:50]
export not account index not found in

[209:52]
config.ts where is it being imported sir

[209:55]
uh

[210:02]
Yes, in

[210:06]
the hell. Uh, cursor account index. Oh,

[210:09]
there you go. Open positions has it.

[210:12]
Remove it. Anything else? Nothing else.

[210:15]
Get add open positions.

[210:18]
Get commit-moved

[210:22]
unused.

[210:25]
Import. Push it.

[210:32]
pull it out.

[210:38]
One index.ts. Does it work now? Still

[210:41]
doesn't work.

[210:44]
Where

[210:47]
is account index being used now?

[210:49]
Config.ts

[210:52]
con account_ind

[210:55]
index equal to one. Are you being used

[210:57]
somewhere, sir? Yes, cancel order.ts.

[211:01]
Okay, that is a very big bug that I

[211:05]
thankfully uh

[211:08]
fixed right now. Else I wouldn't have

[211:10]
been able to cancel orders. Uh which

[211:13]
would have been really sad. Uh

[211:16]
all right, get rid of that. Are you

[211:18]
being used now? No. Great. Get status.

[211:21]
Good thing we fixed that. If not, a lot

[211:23]
of bad things could have happened. I

[211:25]
should add a fail safe. should cancel

[211:26]
all the orders if things go really bad

[211:28]
or something like that. Um,

[211:30]
remove

[211:33]
import report. Yeah, I think I'm playing

[211:35]
with fire over here. I underestimate the

[211:37]
amount of money I've put in. Um, and how

[211:40]
computers can go haywire, especially if

[211:43]
you sleep, which is what I'm going to do

[211:44]
right after. Um, so I should probably

[211:48]
have a fail safe. Um, coded. Okay. If

[211:52]
something goes bad, just cancel all the

[211:53]
orders everywhere. Um,

[211:56]
will I do that though? No, I'm very

[211:58]
sleepy. So, I won't. Uh, let's do a PM2

[212:02]
list. PM2

[212:04]
logs 2. Um, actually one more thing I

[212:07]
forgot. Index again.

[212:09]
Uncomment the set interval. So, this

[212:11]
should run every 5 minutes. Get status.

[212:14]
Get add index.ts. Get commit dashm

[212:21]
add it back

[212:23]
interval

[212:24]
push and then again in the price

[212:28]
tracker. Do we have the interval? We do.

[212:33]
All right. This is pushed.

[212:34]
[clears throat]

[212:36]
Now let me pull again.

[212:40]
And now PM2 list followed by PM2

[212:45]
logs two. Let's see everything good.

[212:46]
Everything looks good here. PM2 start

[212:49]
one.

[212:51]
PM2 logs one. Just tracking the logs.

[212:54]
Seeing everything is good. Everything

[212:55]
looks good until now. These are all

[212:57]
logs. Um, one thing I want to see is in

[213:00]
my database which is this thing right

[213:02]
here. Do we have entries for portfolio

[213:04]
sizes now? Because we are I am running

[213:06]
the thing. Yeah, there we go. we have

[213:08]
entries in the

[213:11]
portfolio size table hopefully getting

[213:13]
added every 2 minutes um because every 2

[213:17]
minutes is when um we had the interval

[213:21]
right if you remember this interval runs

[213:23]
every 2 minutes so hopefully every 2

[213:24]
minutes we're updating this portfolios

[213:26]
table

[213:28]
and as for the actual LLN it'll start in

[213:32]
5 minutes because we have a 5m minute

[213:35]
clock over here so I'll see you guys in

[213:37]
5 minutes um after the first LM call is

[213:40]
made. Uh we'll see if everything is

[213:42]
working or not and is the size being

[213:44]
tracked over time or not.

[213:49]
Right, the thing is running indeed. The

[213:51]
very first invocation has happened as

[213:53]
you can see there was a 15x soul long

[213:56]
that was created. Uh,

[213:59]
and if I go back to

[214:03]
the other sub accounts, let's see what

[214:05]
Claude did. Claude ended up opening a

[214:08]
20x ETH long. Oh boy, a lot of leverage

[214:12]
here. And for the third guy opened a 20x

[214:17]
E long as well. Second and the third,

[214:19]
that's Claw and Deepstick have sort of

[214:21]
been performing very similar. Coin is

[214:22]
taking extremely contrarian bets and is

[214:25]
the most underperforming one at the

[214:26]
moment. This is like a very small number

[214:28]
to look at right now. I'm hitting the

[214:30]
bed which could be a very dumb decision

[214:32]
right now. Um obviously it's 8:30 in the

[214:34]
morning. Um the database is collecting

[214:36]
all the prices over time. Um it is

[214:38]
connecting all the invocations and

[214:40]
thankfully I haven't seen any error yet.

[214:41]
Um we're also running this once every 5

[214:44]
minutes. So I'm not expecting any rate

[214:46]
limits to hit. Um there is some nons

[214:48]
management logic in here which can also

[214:49]
fail from time to time but we're not

[214:51]
using optimistic nonses. Um, which also

[214:54]
means there's a low probability of this

[214:55]
failing. If this ever crashes, the only

[214:57]
problem is we're not rebalancing our

[214:58]
position. Um, and hence, you know, we

[215:00]
might take one position right now, which

[215:02]
if I wake up after 8 hours, uh, might

[215:04]
just be a very bad position to be in.

[215:06]
Um, that's the only thing to worry about

[215:07]
right now. But I think we'll see this in

[215:09]
the morning. Going to hit the bed and

[215:11]
then we'll in the morning write the

[215:12]
front end for it. Visualize how did our

[215:15]
AI perform um, in a span of one day, two

[215:18]
day before the video goes out. I will

[215:20]
see you guys tomorrow. Um, and I might

[215:23]
be regretting or being very happy

[215:24]
tomorrow morning. We'll see. Good night,

[215:26]
folks.

[215:29]
All righty,

[215:31]
the night is over. Did you guys sleep

[215:33]
well? I slept well. I slept for a good 8

[215:35]
hours. Um, and when I woke up like half

[215:38]
an hour ago, all three of the models

[215:40]
were close to $1,000. The surprising

[215:42]
thing was the thing did not stop. Um,

[215:44]
the script was running correctly. There

[215:45]
were no errors. Um, I also woke up in

[215:47]
the middle beach at 2:00 a.m. um or 2

[215:50]
p.m., sorry. Um, and when I woke up, I

[215:52]
saw one of the models was up, the other

[215:54]
two were down. Right now, this is what

[215:55]
it looks like. Um, Claude and Deep Seek

[215:57]
are up by around $40 and $67. Quen is

[216:01]
still at the $1,000, which is very

[216:03]
surprising because Quen is the one

[216:04]
that's performing the best on the

[216:06]
original um Alpha Arena. If you look at

[216:08]
the open positions, um, I'm currently in

[216:10]
the Clawude account. It has a ETH long

[216:13]
that is open that is currently down. And

[216:15]
if you look at

[216:18]
um DeepSk,

[216:20]
it currently has a sole long that is

[216:23]
open that is currently up. Um it's a

[216:25]
really small position that is open over

[216:26]
here though. Um if you look at the trade

[216:28]
history, I think most of the money was

[216:30]
made on a Solana long that both of these

[216:33]
models ended up taking. This took a long

[216:35]
of 8,000 plus,700 plus. Yeah, these so

[216:39]
around $10,000 long that ended up

[216:41]
providing most of the profits over here.

[216:43]
And I think the same order is what

[216:45]
provided most of the profits to the

[216:46]
second one. Um and I think Quen never

[216:48]
really took that position. Um but

[216:50]
surprisingly both of these took the same

[216:52]
position. Um which made most of the

[216:53]
profits to these two guys. Um that's

[216:55]
what it looks like right now. I

[216:57]
hopefully have all the time series data.

[216:59]
Um so what's left now?

[217:02]
Actually, let me just confirm that. Uh

[217:04]
if I go back to my server here,

[217:08]
PM2 list PM2 logs one. This is working

[217:11]
as expected. I see a bunch of errors. I

[217:13]
don't know when they came. Um, but other

[217:16]
than that, this is still working. And,

[217:18]
you know, every 5 minutes sending a

[217:19]
request. And if I look at the logs for

[217:24]
the second script, which is constantly

[217:26]
storing the time series data, it seems

[217:27]
to work. I'll also go to my database

[217:28]
really quickly. Um, and confirm

[217:32]
if we have the time series data for, you

[217:34]
know, the last one day. Um, and if we

[217:36]
do, that pretty much means we are good

[217:40]
to go. We can create a graph out of the

[217:42]
per daily performance and actually see

[217:44]
how things went up and down through the

[217:46]
day. If I look at portfolio size, that

[217:49]
looks fairly filled. Um, if I go to the

[217:51]
next page, next page. Yeah, this looks

[217:55]
decent. Um,

[217:56]
yeah, seems like we have all the time

[217:58]
series data as well. Let's quickly

[218:00]
create a front end that graphs this time

[218:02]
series data out.

[218:08]
All right, I'm skipping through this

[218:10]
part. Took me around 40 minutes to code

[218:12]
it. I w coded most of it. The charting

[218:14]
charting library that the AI ended up

[218:16]
using was recharts. This is what it

[218:18]
looks like right now. It has the graph

[218:20]
on the left that will show you the

[218:21]
performance of every model. Right now,

[218:23]
we're a little bit in the positive.

[218:24]
Nothing crazy. I think $10 or $20 in

[218:26]
profit, but at some point uh Deep Seek

[218:29]
was doing really well. I'm going to

[218:31]
deploy this very soon. So, we'll be able

[218:32]
to track its position over the next one,

[218:34]
two months. I'll keep the capital as is

[218:36]
and we'll see how it performs u in the

[218:38]
time to come. You can also track the

[218:40]
responses from the LM over here and what

[218:42]
tool calls were made u as time goes by.

[218:45]
So just come to this page. I'll share

[218:47]
the live link right now very soon. Um

[218:49]
and you can track how the AI is doing.

[218:51]
The code is open source. You can go to

[218:53]
github.com/hk/itrading

[218:56]
agent um and follow along and figure out

[218:59]
in case you want to deploy this for

[219:00]
yourself. Let me quickly deploy this on

[219:02]
a public facing URL. So you can track

[219:04]
this over time. All right, it's also

[219:07]
deployed. I might make this a little

[219:09]
fancy or ask someone to do it. By the

[219:11]
end, you might see a cleaner page or a

[219:12]
better graph. Uh but this is where you

[219:14]
can go. AI trading.honeyext.com

[219:17]
and you can track its performance over

[219:19]
the upcoming days. And I'm very

[219:21]
surprised by the performance honestly.

[219:23]
Uh I think you and I at least the first

[219:26]
time if you think about AI trading

[219:28]
generally think it'll be a fad. Um

[219:30]
that's what I would expect it to do. Um

[219:32]
but seeing the fact that uh

[219:37]
it's after trading around half a million

[219:40]
dollars u everything is close to up if

[219:43]
not um up it's very surprising for

[219:46]
example for one of these models we're up

[219:47]
$50 um after trading for around half a

[219:50]
mil um which is a very surprising fact.

[219:52]
I personally cannot trade as good as an

[219:54]
AI for sure. She's definitely a better

[219:56]
trader than I am. Um, probably because I

[219:58]
have no idea how to trade. Um, but the

[220:01]
fact that it's profitable is a

[220:03]
surprising stat for me, probably for a

[220:05]
lot of you. Um, does that mean everyone

[220:07]
will trade using AI? I don't know. I

[220:08]
think that's a wrong assumption to make.

[220:10]
And I think over the next few days,

[220:12]
we'll actually see if this is actually a

[220:14]
fad or, you know, we need a lot of data,

[220:16]
much more volumes um, and a lot of more

[220:19]
months and years to actually see if this

[220:21]
is a good thing to do or not. Probably

[220:23]
not something I'd recommend um, with

[220:25]
money that you can't lose. With that,

[220:26]
we'll end it. Hopefully, this was an

[220:28]
interesting video. Um, let me know what

[220:30]
you guys would like to see next. I'll do

[220:31]
one more uh project video soon enough.

[220:34]
Uh, we're trying to aim for two project

[220:35]
videos a month. So, let me know which

[220:37]
one you like to see next. With that,

[220:38]
we'll end it. I'll see you guys in the

[220:39]
next one. Bye-bye.